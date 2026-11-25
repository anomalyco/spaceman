use std::sync::mpsc::Sender;

use crate::app::{WorkerCommand, macos, status_item};

pub(in crate::app) struct WorkerDispatch {
    commands: Sender<WorkerCommand>,
    busy: bool,
    interruptible: bool,
    restore_pending: bool,
    restoring: bool,
    /// The restore under way also undoes the setup tour's arrangement.
    undo: bool,
}

impl WorkerDispatch {
    pub(in crate::app) fn new(commands: Sender<WorkerCommand>) -> Self {
        Self {
            commands,
            busy: false,
            interruptible: false,
            restore_pending: false,
            restoring: false,
            undo: false,
        }
    }

    pub(in crate::app) const fn is_busy(&self) -> bool {
        self.busy
    }

    pub(in crate::app) const fn is_interruptible_busy(&self) -> bool {
        self.busy && self.interruptible
    }

    pub(in crate::app) const fn is_restoring(&self) -> bool {
        self.restoring
    }

    pub(in crate::app) const fn is_restoring_or_pending(&self) -> bool {
        self.restoring || self.restore_pending
    }

    pub(in crate::app) fn send(&mut self, command: WorkerCommand) -> bool {
        let interruptible = command.interruptible();
        if self.commands.send(command).is_err() {
            return false;
        }
        self.busy = true;
        self.interruptible = interruptible;
        true
    }

    pub(in crate::app) fn send_with_outputs(
        &mut self,
        command: impl FnOnce(macos::DisplayOutputs) -> WorkerCommand,
    ) -> bool {
        match macos::outputs() {
            Ok(outputs) => self.send(command(outputs)),
            Err(error) => {
                status_item::set_status(&format!("Display error: {error}"));
                false
            }
        }
    }

    pub(in crate::app) fn send_restore(&mut self) -> bool {
        let outputs = macos::outputs().ok();
        self.busy = self.send(if self.undo {
            WorkerCommand::UndoArrangement(outputs)
        } else {
            WorkerCommand::RestoreAll(outputs)
        });
        self.restoring = self.busy;
        self.busy
    }

    /// The next restore, and its retries, also put back the frames the setup tour's
    /// arrangement changed.
    pub(in crate::app) fn undo_with_next_restore(&mut self) {
        self.undo = true;
    }

    pub(in crate::app) fn request_restore(&mut self) {
        if self.busy {
            self.restore_pending = true;
        } else {
            self.send_restore();
        }
    }

    pub(in crate::app) fn finish_command(&mut self) {
        self.busy = false;
    }

    pub(in crate::app) fn finish_restore(&mut self) {
        self.busy = false;
        self.restoring = false;
        self.undo = false;
    }

    pub(in crate::app) fn defer_restore_retry(&mut self) {
        self.busy = false;
        self.restoring = true;
    }

    pub(in crate::app) fn start_pending_restore(&mut self) -> Option<bool> {
        if !self.restore_pending || self.busy {
            return None;
        }
        self.restore_pending = false;
        Some(self.send_restore())
    }

    pub(in crate::app) fn reset_disconnected(&mut self) {
        self.busy = false;
        self.restore_pending = false;
        self.restoring = false;
        self.undo = false;
    }

    pub(in crate::app) fn stop(&self) {
        drop(self.commands.send(WorkerCommand::Stop));
    }
}
