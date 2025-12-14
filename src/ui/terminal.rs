use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{ActivePanel, App};

/// Draw the terminal window at the bottom of the screen
pub fn draw_terminal(frame: &mut Frame, area: Rect, app: &mut App) {
    // Fill entire area with solid black background using a paragraph
    let filler = Paragraph::new("")
        .style(Style::default().bg(Color::Black))
        .block(Block::default().style(Style::default().bg(Color::Black)));
    frame.render_widget(filler, area);
    
    let (title, shell_output) = match app.active_panel {
        ActivePanel::Left => {
            let output = if let Some(shell) = app.left_shell.as_mut() {
                // Read any available output first if it's a remote shell
                if let crate::shell::ShellType::Remote(remote) = shell {
                    let _ = remote.read_available();
                }
                shell.get_output()
            } else {
                "Shell not available".to_string()
            };
            (" Terminal - Local ", output)
        }
        ActivePanel::Right => {
            let title = if app.remote_connection.is_some() {
                " Terminal - Remote "
            } else {
                " Terminal - Local "
            };
            let output = if let Some(shell) = app.right_shell.as_mut() {
                // Read any available output first if it's a remote shell
                if let crate::shell::ShellType::Remote(remote) = shell {
                    let _ = remote.read_available();
                }
                shell.get_output()
            } else {
                "Shell not available".to_string()
            };
            (title, output)
        }
    };

    // Create border style based on whether we're in input mode
    let border_color = if app.terminal_input_mode {
        Color::Yellow
    } else {
        Color::Green
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Black));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Parse ANSI codes and convert to styled lines
    let lines = parse_ansi_output(&shell_output);

    // Calculate how many lines fit
    let visible_lines_count = inner_area.height as usize;
    let start_idx = lines.len().saturating_sub(visible_lines_count);
    let visible_lines: Vec<Line> = lines.into_iter().skip(start_idx).collect();

    let paragraph = Paragraph::new(visible_lines)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, inner_area);
}

/// Parse ANSI escape codes and convert to ratatui styled text
fn parse_ansi_output(text: &str) -> Vec<Line<'static>> {
    // Create a simple terminal buffer emulator
    let mut buffer = TerminalBuffer::new(200, 1000); // 200 cols, 1000 lines history
    buffer.process(text);
    buffer.to_lines()
}

/// Simple terminal buffer that emulates VT100-style cursor positioning
struct TerminalBuffer {
    lines: Vec<Vec<(char, Style)>>,
    cursor_row: usize,
    cursor_col: usize,
    current_style: Style,
    max_cols: usize,
}

impl TerminalBuffer {
    fn new(max_cols: usize, max_lines: usize) -> Self {
        Self {
            lines: vec![vec![]; max_lines],
            cursor_row: 0,
            cursor_col: 0,
            current_style: Style::default().fg(Color::White),
            max_cols,
        }
    }

    fn ensure_line(&mut self, row: usize) {
        while self.lines.len() <= row {
            self.lines.push(vec![]);
        }
    }

    fn write_char(&mut self, ch: char) {
        self.ensure_line(self.cursor_row);
        
        // Extend line if needed
        while self.lines[self.cursor_row].len() <= self.cursor_col {
            self.lines[self.cursor_row].push((' ', self.current_style));
        }
        
        if self.cursor_col < self.max_cols {
            if self.cursor_col < self.lines[self.cursor_row].len() {
                self.lines[self.cursor_row][self.cursor_col] = (ch, self.current_style);
            } else {
                self.lines[self.cursor_row].push((ch, self.current_style));
            }
            self.cursor_col += 1;
        }
    }

    fn process(&mut self, text: &str) {
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Peek to see what kind of escape sequence
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                    let mut params = String::new();
                    
                    // Collect all parameter characters (digits, semicolons, ?)
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() || c == ';' || c == '?' {
                            params.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    
                    // Get command character
                    if let Some(cmd) = chars.next() {
                        self.handle_csi(&params, cmd);
                    }
                } else if chars.peek() == Some(&']') {
                    // OSC sequence
                    chars.next(); // consume ']'
                    let mut prev = ' ';
                    while let Some(c) = chars.next() {
                        if c == '\x07' || (prev == '\x1b' && c == '\\') {
                            break;
                        }
                        prev = c;
                    }
                } else if chars.peek() == Some(&'(') || chars.peek() == Some(&')') {
                    // Character set designation
                    chars.next(); // consume '(' or ')'
                    chars.next(); // consume character set ID
                } else if chars.peek() == Some(&'=') || chars.peek() == Some(&'>') {
                    // Keypad mode
                    chars.next();
                } else {
                    // Unknown escape - consume next char if present
                    chars.next();
                }
            } else if ch == '\n' {
                self.cursor_row += 1;
                self.cursor_col = 0;
            } else if ch == '\r' {
                self.cursor_col = 0;
            } else if ch == '\x08' {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            } else if ch == '\t' {
                let spaces = 8 - (self.cursor_col % 8);
                for _ in 0..spaces {
                    self.write_char(' ');
                }
            } else if !ch.is_control() {
                self.write_char(ch);
            }
        }
    }

    fn handle_csi(&mut self, params: &str, cmd: char) {
        match cmd {
            'm' => {
                self.current_style = parse_sgr_codes(params, self.current_style);
            }
            'H' | 'f' => {
                // Cursor position
                let parts: Vec<&str> = params.split(';').collect();
                let row = parts.get(0).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1).saturating_sub(1);
                let col = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1).saturating_sub(1);
                self.cursor_row = row;
                self.cursor_col = col;
            }
            'A' => {
                // Cursor up
                let n = params.parse::<usize>().unwrap_or(1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            'B' => {
                // Cursor down
                let n = params.parse::<usize>().unwrap_or(1);
                self.cursor_row += n;
            }
            'C' => {
                // Cursor forward
                let n = params.parse::<usize>().unwrap_or(1);
                self.cursor_col = (self.cursor_col + n).min(self.max_cols - 1);
            }
            'D' => {
                // Cursor back
                let n = params.parse::<usize>().unwrap_or(1);
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            'J' => {
                // Erase display
                let mode = params.parse::<usize>().unwrap_or(0);
                match mode {
                    0 => {
                        // Clear from cursor to end
                        if self.cursor_row < self.lines.len() {
                            self.lines[self.cursor_row].truncate(self.cursor_col);
                            for i in (self.cursor_row + 1)..self.lines.len() {
                                self.lines[i].clear();
                            }
                        }
                    }
                    2 => {
                        // Clear entire screen
                        for line in &mut self.lines {
                            line.clear();
                        }
                        self.cursor_row = 0;
                        self.cursor_col = 0;
                    }
                    _ => {}
                }
            }
            'K' => {
                // Erase line
                let mode = params.parse::<usize>().unwrap_or(0);
                if self.cursor_row < self.lines.len() {
                    match mode {
                        0 => {
                            // Clear from cursor to end of line
                            self.lines[self.cursor_row].truncate(self.cursor_col);
                        }
                        1 => {
                            // Clear from start to cursor
                            for i in 0..self.cursor_col.min(self.lines[self.cursor_row].len()) {
                                self.lines[self.cursor_row][i] = (' ', self.current_style);
                            }
                        }
                        2 => {
                            // Clear entire line
                            self.lines[self.cursor_row].clear();
                        }
                        _ => {}
                    }
                }
            }
            'h' | 'l' => {
                // Mode set (h) / reset (l) - including DEC private modes (starting with ?)
                // Examples: ?2004h (bracketed paste), ?1h (application cursor keys)
                // We ignore these but need to consume them
            }
            's' | 'u' => {
                // Save (s) / restore (u) cursor position - ignore
            }
            'r' => {
                // Set scrolling region - ignore
            }
            'G' => {
                // Cursor horizontal absolute
                let col = params.parse::<usize>().unwrap_or(1).saturating_sub(1);
                self.cursor_col = col.min(self.max_cols - 1);
            }
            'd' => {
                // Line position absolute
                let row = params.parse::<usize>().unwrap_or(1).saturating_sub(1);
                self.cursor_row = row;
            }
            _ => {
                // Ignore all other CSI commands
            }
        }
    }

    fn to_lines(self) -> Vec<Line<'static>> {
        let mut result = Vec::new();
        
        for line in &self.lines {
            if line.is_empty() {
                continue;
            }
            
            let mut spans = Vec::new();
            let mut current_text = String::new();
            let mut current_style = Style::default().fg(Color::White);
            
            for &(ch, style) in line {
                if style != current_style {
                    if !current_text.is_empty() {
                        spans.push(Span::styled(current_text.clone(), current_style));
                        current_text.clear();
                    }
                    current_style = style;
                }
                current_text.push(ch);
            }
            
            if !current_text.is_empty() {
                spans.push(Span::styled(current_text, current_style));
            }
            
            if !spans.is_empty() {
                result.push(Line::from(spans));
            }
        }
        
        result
    }
}

/// Parse SGR (Select Graphic Rendition) codes and update style
fn parse_sgr_codes(codes: &str, mut style: Style) -> Style {
    if codes.is_empty() || codes == "0" {
        // Reset
        return Style::default().fg(Color::White);
    }
    
    let parts: Vec<&str> = codes.split(';').collect();
    let mut i = 0;
    
    while i < parts.len() {
        match parts[i] {
            "0" => style = Style::default().fg(Color::White),
            "1" => style = style.add_modifier(ratatui::style::Modifier::BOLD),
            "4" => style = style.add_modifier(ratatui::style::Modifier::UNDERLINED),
            "7" => style = style.add_modifier(ratatui::style::Modifier::REVERSED),
            "30" => style = style.fg(Color::Black),
            "31" => style = style.fg(Color::Red),
            "32" => style = style.fg(Color::Green),
            "33" => style = style.fg(Color::Yellow),
            "34" => style = style.fg(Color::Blue),
            "35" => style = style.fg(Color::Magenta),
            "36" => style = style.fg(Color::Cyan),
            "37" => style = style.fg(Color::White),
            "90" => style = style.fg(Color::DarkGray),
            "91" => style = style.fg(Color::LightRed),
            "92" => style = style.fg(Color::LightGreen),
            "93" => style = style.fg(Color::LightYellow),
            "94" => style = style.fg(Color::LightBlue),
            "95" => style = style.fg(Color::LightMagenta),
            "96" => style = style.fg(Color::LightCyan),
            "97" => style = style.fg(Color::Gray),
            "40" => style = style.bg(Color::Black),
            "41" => style = style.bg(Color::Red),
            "42" => style = style.bg(Color::Green),
            "43" => style = style.bg(Color::Yellow),
            "44" => style = style.bg(Color::Blue),
            "45" => style = style.bg(Color::Magenta),
            "46" => style = style.bg(Color::Cyan),
            "47" => style = style.bg(Color::White),
            "38" => {
                // Extended foreground color
                if i + 2 < parts.len() && parts[i + 1] == "5" {
                    // 256 color mode
                    if let Ok(color_idx) = parts[i + 2].parse::<u8>() {
                        style = style.fg(Color::Indexed(color_idx));
                    }
                    i += 2;
                }
            }
            "48" => {
                // Extended background color
                if i + 2 < parts.len() && parts[i + 1] == "5" {
                    // 256 color mode
                    if let Ok(color_idx) = parts[i + 2].parse::<u8>() {
                        style = style.bg(Color::Indexed(color_idx));
                    }
                    i += 2;
                }
            }
            _ => {} // Ignore unknown codes
        }
        i += 1;
    }
    
    style
}