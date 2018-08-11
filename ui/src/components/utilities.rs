/*
 * meli - ui crate.
 *
 * Copyright 2017-2018 Manos Pitsidianakis
 *
 * This file is part of meli.
 *
 * meli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * meli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with meli. If not, see <http://www.gnu.org/licenses/>.
 */

/*! Various useful components that can be used in a generic fashion.
 */
use super::*;

/// A horizontally split in half container.
pub struct HSplit {
    top: Entity,
    bottom: Entity,
    show_divider: bool,
    ratio: usize, // bottom/whole height * 100
}

impl fmt::Display for HSplit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO display subject/info
        self.top.fmt(f)
    }
}

impl HSplit {
    pub fn new(top: Entity, bottom: Entity, ratio: usize, show_divider: bool) -> Self {
        HSplit {
            top,
            bottom,
            show_divider,
            ratio,
        }
    }
}

impl Component for HSplit {
    fn draw(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        if !is_valid_area!(area) {
            return;
        }
        let upper_left = upper_left!(area);
        let bottom_right = bottom_right!(area);
        let total_rows = get_y(bottom_right) - get_y(upper_left);
        let bottom_entity_height = (self.ratio * total_rows) / 100;
        let mid = get_y(upper_left) + total_rows - bottom_entity_height;

        if self.show_divider {
            for i in get_x(upper_left)..=get_x(bottom_right) {
                grid[(i, mid)].set_ch('─');
            }
        }
        self.top.component.draw(
            grid,
            (
                upper_left,
                (get_x(bottom_right), get_y(upper_left) + mid - 1),
            ),
            context,
        );
        self.bottom.component.draw(
            grid,
            ((get_x(upper_left), get_y(upper_left) + mid), bottom_right),
            context,
        );
    }
    fn process_event(&mut self, event: &UIEvent, context: &mut Context) {
        self.top.rcv_event(event, context);
        self.bottom.rcv_event(event, context);
    }
    fn is_dirty(&self) -> bool {
        self.top.component.is_dirty() || self.bottom.component.is_dirty()
    }
    fn set_dirty(&mut self) {
        self.top.component.set_dirty();
        self.bottom.component.set_dirty();
    }
}

/// A vertically split in half container.
pub struct VSplit {
    left: Entity,
    right: Entity,
    show_divider: bool,
    /// This is the width of the right container to the entire width.
    ratio: usize, // right/(container width) * 100
}

impl fmt::Display for VSplit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO display focused entity
        self.right.fmt(f)
    }
}

impl VSplit {
    pub fn new(left: Entity, right: Entity, ratio: usize, show_divider: bool) -> Self {
        VSplit {
            left,
            right,
            show_divider,
            ratio,
        }
    }
}

impl Component for VSplit {
    fn draw(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        if !is_valid_area!(area) {
            return;
        }
        let upper_left = upper_left!(area);
        let bottom_right = bottom_right!(area);
        let total_cols = get_x(bottom_right) - get_x(upper_left);
        let right_entity_width = (self.ratio * total_cols) / 100;
        let mid = get_x(bottom_right) - right_entity_width;

        if get_y(upper_left) > 1 {
            let c = grid
                .get(mid, get_y(upper_left) - 1)
                .map(|a| a.ch())
                .unwrap_or_else(|| ' ');
            if let HORZ_BOUNDARY = c {
                grid[(mid, get_y(upper_left) - 1)].set_ch(LIGHT_DOWN_AND_HORIZONTAL);
            }
        }

        if self.show_divider {
            for i in get_y(upper_left)..=get_y(bottom_right) {
                grid[(mid, i)].set_ch(VERT_BOUNDARY);
                grid[(mid, i)].set_fg(Color::Default);
                grid[(mid, i)].set_bg(Color::Default);
            }
            if get_y(bottom_right) > 1 {
                let c = grid
                    .get(mid, get_y(bottom_right) - 1)
                    .map(|a| a.ch())
                    .unwrap_or_else(|| ' ');
                match c {
                    HORZ_BOUNDARY => {
                        grid[(mid, get_y(bottom_right) + 1)].set_ch(LIGHT_UP_AND_HORIZONTAL);
                    }
                    _ => {}
                }
            }
        }
        self.left
            .component
            .draw(grid, (upper_left, (mid - 1, get_y(bottom_right))), context);
        self.right
            .component
            .draw(grid, ((mid + 1, get_y(upper_left)), bottom_right), context);
    }
    fn process_event(&mut self, event: &UIEvent, context: &mut Context) {
        self.left.rcv_event(event, context);
        self.right.rcv_event(event, context);
    }
    fn is_dirty(&self) -> bool {
        self.left.component.is_dirty() || self.right.component.is_dirty()
    }
    fn set_dirty(&mut self) {
        self.left.component.set_dirty();
        self.right.component.set_dirty();
    }
}

/// A pager for text.
/// `Pager` holds its own content in its own `CellBuffer` and when `draw` is called, it draws the
/// current view of the text. It is responsible for scrolling etc.
pub struct Pager {
    cursor_pos: usize,
    height: usize,
    width: usize,
    dirty: bool,
    content: CellBuffer,
}

impl fmt::Display for Pager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO display info
        write!(f, "pager")
    }
}

impl Pager {
    pub fn from_string(mut text: String, context: &mut Context, cursor_pos: Option<usize>) -> Self {
        let pager_filter: Option<&String> = context.settings.pager.filter.as_ref();
        //let format_flowed: bool = context.settings.pager.format_flowed;
        if let Some(bin) = pager_filter {
            use std::io::Write;
            use std::process::{Command, Stdio};
            let mut filter_child = Command::new(bin)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to start pager filter process");
            {
                let mut stdin = filter_child.stdin.as_mut().expect("failed to open stdin");
                stdin
                    .write_all(text.as_bytes())
                    .expect("Failed to write to stdin");
            }

            text = String::from_utf8_lossy(
                &filter_child
                    .wait_with_output()
                    .expect("Failed to wait on filter")
                    .stdout,
            ).to_string();
        }

        let lines: Vec<&str> = text.trim().split('\n').collect();
        let height = lines.len() + 1;
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let mut content = CellBuffer::new(width, height, Cell::with_char(' '));
        //interpret_format_flowed(&text);
        Pager::print_string(&mut content, &text);
        Pager {
            cursor_pos: cursor_pos.unwrap_or(0),
            height: height,
            width: width,
            dirty: true,
            content: content,
        }
    }
    pub fn from_str(s: &str, cursor_pos: Option<usize>) -> Self {
        let lines: Vec<&str> = s.trim().split('\n').collect();
        let height = lines.len();
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let mut content = CellBuffer::new(width, height, Cell::with_char(' '));
        Pager::print_string(&mut content, s);
        Pager {
            cursor_pos: cursor_pos.unwrap_or(0),
            height,
            width,
            dirty: true,
            content,
        }
    }
    pub fn from_buf(buf: &CellBuffer, cursor_pos: Option<usize>) -> Self {
        let lines: Vec<&[Cell]> = buf.split(|cell| cell.ch() == '\n').collect();
        let height = lines.len();
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0) + 1;
        let mut content = CellBuffer::new(width, height, Cell::with_char(' '));
        {
            let mut x;
            let c_slice: &mut [Cell] = &mut content;
            for (y, l) in lines.iter().enumerate() {
                let y_r = y * width;
                x = l.len() + y_r;
                c_slice[y_r..x].copy_from_slice(l);
                c_slice[x].set_ch('\n');
            }
        }
        Pager {
            cursor_pos: cursor_pos.unwrap_or(0),
            height,
            width,
            dirty: true,
            content,
        }
    }
    pub fn print_string(content: &mut CellBuffer, s: &str) {
        let lines: Vec<&str> = s.trim().split('\n').collect();
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        if width > 0 {
            for (i, l) in lines.iter().enumerate() {
                write_string_to_grid(
                    l,
                    content,
                    Color::Default,
                    Color::Default,
                    ((0, i), (width - 1, i)),
                    true,
                );
            }
        }
    }
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }
}

impl Component for Pager {
    fn draw(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        if !is_valid_area!(area) {
            return;
        }
        if !self.is_dirty() {
            return;
        }

        self.dirty = false;
        if self.cursor_pos > 0 && self.cursor_pos + 1 + height!(area) > self.height {
            self.cursor_pos = self.cursor_pos.saturating_sub(1);
            return;
        }

        if self.height == 0 || self.height == self.cursor_pos || self.width == 0 {
            return;
        }

        clear_area(grid, area);
        //let pager_context: usize = context.settings.pager.pager_context;
        //let pager_stop: bool = context.settings.pager.pager_stop;
        //let rows = y(bottom_right) - y(upper_left);
        //let page_length = rows / self.height;
        copy_area_with_break(
            grid,
            &self.content,
            area,
            ((0, self.cursor_pos), (self.width - 1, self.height - 1)),
        );
        context.dirty_areas.push_back(area);
    }
    fn process_event(&mut self, event: &UIEvent, _context: &mut Context) {
        match event.event_type {
            UIEventType::Input(Key::Char('k')) => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.dirty = true;
                }
            }
            UIEventType::Input(Key::Char('j')) => {
                if self.height > 0 && self.cursor_pos + 1 < self.height {
                    self.cursor_pos += 1;
                    self.dirty = true;
                }
            }
            UIEventType::ChangeMode(UIMode::Normal) => {
                self.dirty = true;
            }
            UIEventType::Resize => {
                self.dirty = true;
            }
            _ => {}
        }
    }
    fn is_dirty(&self) -> bool {
        self.dirty
    }
    fn set_dirty(&mut self) {
        self.dirty = true;
    }
}

/// Status bar.
pub struct StatusBar {
    container: Entity,
    status: String,
    notifications: VecDeque<String>,
    ex_buffer: String,
    mode: UIMode,
    height: usize,
    dirty: bool,
}

impl fmt::Display for StatusBar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO display info
        write!(f, "status bar")
    }
}

impl StatusBar {
    pub fn new(container: Entity) -> Self {
        StatusBar {
            container,
            status: String::with_capacity(256),
            notifications: VecDeque::new(),
            ex_buffer: String::with_capacity(256),
            dirty: true,
            mode: UIMode::Normal,
            height: 1,
        }
    }
    fn draw_status_bar(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        clear_area(grid, area);
        if let Some(n) = self.notifications.pop_front() {
            self.dirty = true;
            write_string_to_grid(&n, grid, Color::Byte(219), Color::Byte(88), area, false);
        } else {
            write_string_to_grid(
                &self.status,
                grid,
                Color::Byte(123),
                Color::Byte(26),
                area,
                false,
            );
        }
        change_colors(grid, area, Color::Byte(123), Color::Byte(26));
        context.dirty_areas.push_back(area);
    }
    fn draw_execute_bar(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        clear_area(grid, area);
        write_string_to_grid(
            &self.ex_buffer,
            grid,
            Color::Byte(219),
            Color::Byte(88),
            area,
            false,
        );
        change_colors(grid, area, Color::Byte(219), Color::Byte(88));
        context.dirty_areas.push_back(area);
    }
}

impl Component for StatusBar {
    fn draw(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        if !is_valid_area!(area) {
            return;
        }
        let upper_left = upper_left!(area);
        let bottom_right = bottom_right!(area);

        let total_rows = get_y(bottom_right) - get_y(upper_left);
        if total_rows <= self.height {
            return;
        }
        let height = self.height;

        self.container.component.draw(
            grid,
            (
                upper_left,
                (get_x(bottom_right), get_y(bottom_right) - height),
            ),
            context,
        );

        if !self.is_dirty() {
            return;
        }
        self.dirty = false;
        self.draw_status_bar(
            grid,
            (set_y(upper_left, get_y(bottom_right)), bottom_right),
            context,
        );
        match self.mode {
            UIMode::Normal => {}
            UIMode::Execute => {
                self.draw_execute_bar(
                    grid,
                    (
                        set_y(upper_left, get_y(bottom_right) - height + 1),
                        set_y(bottom_right, get_y(bottom_right) - height + 1),
                    ),
                    context,
                );
            }
            _ => {}
        }
    }
    fn process_event(&mut self, event: &UIEvent, context: &mut Context) {
        self.container.rcv_event(event, context);
        match &event.event_type {
            UIEventType::RefreshMailbox((ref idx_a, ref idx_f)) => {
                match context.accounts[*idx_a].status(*idx_f) {
                    Ok(()) => {}
                    Err(_) => {
                        return;
                    }
                }
                let m = &context.accounts[*idx_a][*idx_f].as_ref().unwrap();
                self.status = format!(
                    "{} | Mailbox: {}, Messages: {}, New: {}",
                    self.mode,
                    m.folder.name(),
                    m.collection.len(),
                    m.collection.iter().filter(|e| !e.is_seen()).count()
                );
                self.dirty = true;
            }
            UIEventType::ChangeMode(m) => {
                let offset = self.status.find('|').unwrap_or_else(|| self.status.len());
                self.status.replace_range(..offset, &format!("{} ", m));
                self.dirty = true;
                self.mode = *m;
                match m {
                    UIMode::Normal => {
                        self.height = 1;
                        if !self.ex_buffer.is_empty() {
                            context.replies.push_back(UIEvent {
                                id: 0,
                                event_type: UIEventType::Command(self.ex_buffer.clone()),
                            });
                        }
                        self.ex_buffer.clear()
                    }
                    UIMode::Execute => {
                        self.height = 2;
                    }
                    _ => {}
                };
            }
            UIEventType::ExInput(Key::Char(c)) => {
                self.dirty = true;
                self.ex_buffer.push(*c);
            }
            UIEventType::ExInput(Key::Ctrl('u')) => {
                self.dirty = true;
                self.ex_buffer.clear();
            }
            UIEventType::ExInput(Key::Backspace) | UIEventType::ExInput(Key::Ctrl('h')) => {
                self.dirty = true;
                self.ex_buffer.pop();
            }
            UIEventType::Resize => {
                self.dirty = true;
            }
            UIEventType::StatusNotification(s) => {
                self.notifications.push_back(s.clone());
                self.dirty = true;
            }
            _ => {}
        }
    }
    fn is_dirty(&self) -> bool {
        self.dirty || self.container.component.is_dirty()
    }
    fn set_dirty(&mut self) {
        self.dirty = true;
    }
}

// A box with a text content.
pub struct TextBox {
    _content: String,
}

impl TextBox {
    pub fn new(s: String) -> Self {
        TextBox { _content: s }
    }
}

impl fmt::Display for TextBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO display info
        write!(f, "text box")
    }
}

impl Component for TextBox {
    fn draw(&mut self, _grid: &mut CellBuffer, _area: Area, _context: &mut Context) {}
    fn process_event(&mut self, _event: &UIEvent, _context: &mut Context) {}
    fn set_dirty(&mut self) {}
}

pub struct Progress {
    description: String,
    total_work: usize,
    finished: usize,
}

impl Progress {
    pub fn new(s: String, total_work: usize) -> Self {
        Progress {
            description: s,
            total_work,
            finished: 0,
        }
    }

    pub fn add_work(&mut self, n: usize) -> () {
        if self.finished >= self.total_work {
            return;
        }
        self.finished += n;
    }

    pub fn percentage(&self) -> usize {
        if self.total_work > 0 {
            100 * self.finished / self.total_work
        } else {
            0
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

impl fmt::Display for Progress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO display info
        write!(f, "progress bar")
    }
}

impl Component for Progress {
    fn draw(&mut self, _grid: &mut CellBuffer, _area: Area, _context: &mut Context) {
        unimplemented!()
    }
    fn process_event(&mut self, _event: &UIEvent, _context: &mut Context) {
        return;
    }
    fn set_dirty(&mut self) {}
}

pub struct Tabbed {
    children: Vec<Box<Component>>,
    cursor_pos: usize,
}

impl Tabbed {
    pub fn new(children: Vec<Box<Component>>) -> Self {
        Tabbed {
            children,
            cursor_pos: 0,
        }
    }
    fn draw_tabs(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        let mut x = get_x(upper_left!(area));
        let mut y: usize = get_y(upper_left!(area));
        for (idx, c) in self.children.iter().enumerate() {
            let (fg, bg) = if idx == self.cursor_pos {
                (Color::Default, Color::Default)
            } else {
                (Color::Byte(15), Color::Byte(8))
            };
            let (x_, _y_) = write_string_to_grid(
                &format!(" {} ", c),
                grid,
                fg,
                bg,
                (set_x(upper_left!(area), x), bottom_right!(area)),
                false,
            );
            x = x_ + 1;
            if y != _y_ {
                break;
            }
            y = _y_;
        }
        let (cols, _) = grid.size();
        let cslice: &mut [Cell] = grid;
        for c in cslice[(y * cols) + x..(y * cols) + cols].iter_mut() {
            c.set_bg(Color::Byte(7));
        }
        context.dirty_areas.push_back(area);
    }
    pub fn add_component(&mut self, new: Box<Component>) {
        self.children.push(new);
    }
}

impl fmt::Display for Tabbed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO display info
        write!(f, "tabs")
    }
}

impl Component for Tabbed {
    fn draw(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        if self.children.len() > 1 {
            self.draw_tabs(
                grid,
                (
                    upper_left!(area),
                    set_x(upper_left!(area), get_x(bottom_right!(area))),
                ),
                context,
            );
            let y = get_y(upper_left!(area));
            self.children[self.cursor_pos].draw(
                grid,
                (set_y(upper_left!(area), y + 1), bottom_right!(area)),
                context,
            );
        } else {
            self.children[self.cursor_pos].draw(grid, area, context);
        }
    }
    fn process_event(&mut self, event: &UIEvent, context: &mut Context) {
        match &event.event_type {
            UIEventType::Input(Key::Char('T')) => {
                self.cursor_pos = (self.cursor_pos + 1) % self.children.len();
                self.children[self.cursor_pos].set_dirty();
                return;
            }
            _ => {}
        }
        self.children[self.cursor_pos].process_event(event, context);
    }
    fn is_dirty(&self) -> bool {
        self.children[self.cursor_pos].is_dirty()
    }
    fn set_dirty(&mut self) {}
}
