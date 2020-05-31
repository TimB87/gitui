use super::{
    filetree::FileTreeComponent,
    utils::filetree::{FileTreeItem, FileTreeItemKind},
    CommandBlocking, DrawableComponent,
};
use crate::{
    components::{CommandInfo, Component},
    keys,
    queue::{Action, InternalEvent, NeedsUpdate, Queue, ResetItem},
    strings,
    ui::style::Theme,
};
use anyhow::Result;
use asyncgit::{sync, StatusItem, StatusItemType, CWD};
use crossterm::event::Event;
use std::path::Path;
use std::process::Command;
use strings::commands;
use tui::{backend::Backend, layout::Rect, Frame};

///
pub struct ChangesComponent {
    files: FileTreeComponent,
    is_working_dir: bool,
    queue: Queue,
}

impl ChangesComponent {
    ///
    pub fn new(
        title: &str,
        focus: bool,
        is_working_dir: bool,
        queue: Queue,
        theme: &Theme,
    ) -> Self {
        Self {
            files: FileTreeComponent::new(
                title,
                focus,
                queue.clone(),
                theme,
            ),
            is_working_dir,
            queue,
        }
    }

    ///
    pub fn update(&mut self, list: &[StatusItem]) -> Result<()> {
        self.files.update(list)?;

        Ok(())
    }

    ///
    pub fn selection(&self) -> Option<FileTreeItem> {
        self.files.selection()
    }

    ///
    pub fn focus_select(&mut self, focus: bool) {
        self.files.focus_select(focus)
    }

    /// returns true if list is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    ///
    pub fn is_file_seleted(&self) -> bool {
        self.files.is_file_seleted()
    }

    fn index_add_remove(&mut self) -> Result<bool> {
        if let Some(tree_item) = self.selection() {
            if self.is_working_dir {
                if let FileTreeItemKind::File(i) = tree_item.kind {
                    if let Some(status) = i.status {
                        let path = Path::new(i.path.as_str());
                        match status {
                            StatusItemType::Deleted => {
                                sync::stage_addremoved(CWD, path)?
                            }
                            _ => sync::stage_add_file(CWD, path)?,
                        };

                        return Ok(true);
                    }
                } else {
                    //TODO: check if we can handle the one file case with it aswell
                    sync::stage_add_all(
                        CWD,
                        tree_item.info.full_path.as_str(),
                    )?;

                    return Ok(true);
                }
            } else {
                let path =
                    Path::new(tree_item.info.full_path.as_str());
                sync::reset_stage(CWD, path)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn dispatch_reset_workdir(&mut self) -> bool {
        if let Some(tree_item) = self.selection() {
            let is_folder =
                matches!(tree_item.kind, FileTreeItemKind::Path(_));
            self.queue.borrow_mut().push_back(
                InternalEvent::ConfirmAction(Action::Reset(
                    ResetItem {
                        path: tree_item.info.full_path,
                        is_folder,
                    },
                )),
            );

            return true;
        }
        false
    }
}

impl DrawableComponent for ChangesComponent {
    fn draw<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        r: Rect,
    ) -> Result<()> {
        self.files.draw(f, r)?;

        Ok(())
    }
}

impl Component for ChangesComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        self.files.commands(out, force_all);

        let some_selection = self.selection().is_some();

        if self.is_working_dir {
            out.push(CommandInfo::new(
                commands::STAGE_ITEM,
                some_selection,
                self.focused(),
            ));
            out.push(CommandInfo::new(
                commands::RESET_ITEM,
                some_selection,
                self.focused(),
            ));
        } else {
            out.push(CommandInfo::new(
                commands::UNSTAGE_ITEM,
                some_selection,
                self.focused(),
            ));
            out.push(
                CommandInfo::new(
                    commands::COMMIT_OPEN,
                    !self.is_empty(),
                    self.focused() || force_all,
                )
                .order(-1),
            );
        }

        CommandBlocking::PassingOn
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.files.event(ev)? {
            return Ok(true);
        }

        if self.focused() {
            if let Event::Key(e) = ev {
                return match e {
                    keys::OPEN_COMMIT
                        if !self.is_working_dir
                            && !self.is_empty() =>
                    {
                        self.queue
                            .borrow_mut()
                            .push_back(InternalEvent::OpenCommit);
                        Ok(true)
                    }
                    keys::STATUS_STAGE_FILE => {
                        if self.index_add_remove()? {
                            self.queue.borrow_mut().push_back(
                                InternalEvent::Update(
                                    NeedsUpdate::ALL,
                                ),
                            );
                        }
                        Ok(true)
                    }
                    keys::PUSH => {
                        Command::new("sh")
                            .arg("-c")
                            .arg("git push")
                            .output()
                            .unwrap();
                        Ok(true)
                    }
                    keys::STATUS_RESET_FILE
                        if self.is_working_dir =>
                    {
                        Ok(self.dispatch_reset_workdir())
                    }
                    _ => Ok(false),
                };
            }
        }

        Ok(false)
    }

    fn focused(&self) -> bool {
        self.files.focused()
    }
    fn focus(&mut self, focus: bool) {
        self.files.focus(focus)
    }
}
