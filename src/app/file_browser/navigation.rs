use super::{BrowserFocus, FileBrowser, Outcome};
use anyhow::Result;
use crossterm::event::{Event, KeyCode as K, KeyEvent, KeyModifiers as M};
use ratatui_explorer::Input;

impl FileBrowser {
    pub(super) fn handle_key(&mut self, event: KeyEvent) -> Result<Outcome> {
        if self.overwrite.is_some() {
            return Ok(match event.code {
                K::Char('y') | K::Char('Y') => Outcome::Selected(self.overwrite.take().unwrap()),
                K::Char('n') | K::Char('N') | K::Esc => {
                    self.overwrite = None;
                    Outcome::Continue
                }
                _ => Outcome::Continue,
            });
        }
        self.error = None;
        match event.code {
            K::Esc => return Ok(Outcome::Cancel),
            K::Tab | K::BackTab => {
                self.focus = if self.focus == BrowserFocus::List {
                    BrowserFocus::Path
                } else {
                    BrowserFocus::List
                };
                self.selected_text = true;
            }
            K::Char('l') if event.modifiers.contains(M::CONTROL) => {
                self.focus = BrowserFocus::Path;
                self.selected_text = true;
            }
            K::F(5) => {
                let selected = self
                    .explorer
                    .files()
                    .get(self.explorer.selected_idx())
                    .map(|f| f.path.clone());
                self.explorer.set_cwd(self.explorer.cwd().clone())?;
                if let Some(index) = self
                    .explorer
                    .files()
                    .iter()
                    .position(|f| Some(&f.path) == selected.as_ref())
                {
                    self.explorer.set_selected_idx(index);
                }
            }
            K::Up | K::Down | K::PageUp | K::PageDown => {
                self.focus = BrowserFocus::List;
                self.navigate(Input::from(&Event::Key(event)))?;
            }
            _ if self.focus == BrowserFocus::Path => return self.edit_path(event),
            K::Enter => {
                if let Some(file) = self.explorer.files().get(self.explorer.selected_idx()) {
                    if file.is_dir {
                        self.navigate(Input::Right)?;
                        return Ok(Outcome::Continue);
                    }
                    return self.submit(file.path.clone());
                }
            }
            K::Char('.') => self
                .explorer
                .set_show_hidden(!self.explorer.show_hidden())?,
            _ => self.navigate(Input::from(&Event::Key(event)))?,
        }
        Ok(Outcome::Continue)
    }

    fn edit_path(&mut self, event: KeyEvent) -> Result<Outcome> {
        let ctrl = event.modifiers.contains(M::CONTROL);
        match event.code {
            K::Enter => return self.submit(self.typed_path()?),
            K::Char('a') if ctrl => self.selected_text = true,
            K::Char(c) if !ctrl && !event.modifiers.contains(M::ALT) => {
                if self.selected_text {
                    self.input.clear();
                    self.selected_text = false;
                }
                self.input.push(c);
            }
            K::Backspace | K::Delete => {
                if self.selected_text {
                    self.input.clear();
                } else {
                    self.input.pop();
                }
                self.selected_text = false;
            }
            _ => {}
        }
        Ok(Outcome::Continue)
    }

    fn navigate(&mut self, input: Input) -> Result<()> {
        if matches!(input, Input::None) {
            return Ok(());
        }
        match input {
            // Avoid the explorer's swap_remove on failed directory entry.
            Input::Right => {
                if let Some(file) = self.explorer.files().get(self.explorer.selected_idx()) {
                    if file.is_dir {
                        self.explorer.set_cwd(file.path.clone())?;
                    }
                }
            }
            Input::Left => {
                if let Some(parent) = self.explorer.cwd().parent() {
                    self.explorer.set_cwd(parent.to_owned())?;
                }
            }
            _ if !self.explorer.files().is_empty() => self.explorer.handle(input)?,
            _ => {}
        }
        if let Some(file) = self.explorer.files().get(self.explorer.selected_idx()) {
            if !file.is_dir {
                self.input = file
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                self.selected_text = true;
            }
        }
        Ok(())
    }
}
