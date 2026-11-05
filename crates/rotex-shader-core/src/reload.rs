/// Host-side swap of a cooked shader at a frame boundary.
///
/// [`Self::stage`] keeps the candidate off the frame path. [`Self::commit`]
/// swaps it in only when the caller reports that the GPU is idle for the
/// previous shader and that the new layout is compatible. An incompatible
/// candidate is dropped and the live shader stays. A compatible candidate
/// stays staged until the GPU is idle, so the caller can retry on a later frame.
/// The caller retires the previous GPU pipeline only after `commit` returns
/// true, and only after the fence for the last frame that used it.
#[derive(Debug)]
pub struct ReloadTransaction<T> {
    live: T,
    staged: Option<T>,
}

impl<T> ReloadTransaction<T> {
    /// Starts a transaction whose live value is `live`.
    pub fn new(live: T) -> Self {
        Self { live, staged: None }
    }

    /// Returns the shader currently in use.
    pub fn live(&self) -> &T {
        &self.live
    }

    /// Returns the candidate waiting for a frame boundary.
    pub fn staged(&self) -> Option<&T> {
        self.staged.as_ref()
    }

    /// Replaces any staged candidate. The live shader is unchanged.
    pub fn stage(&mut self, next: T) {
        self.staged = Some(next);
    }

    /// Swaps the staged shader in, or keeps the live shader.
    ///
    /// Returns true only when a candidate was committed. `layout_compatible`
    /// false drops the candidate. `gpu_idle` false leaves a compatible
    /// candidate staged.
    pub fn commit(&mut self, gpu_idle: bool, layout_compatible: bool) -> bool {
        if self.staged.is_none() {
            return false;
        }
        if !layout_compatible {
            self.staged = None;
            return false;
        }
        if !gpu_idle {
            return false;
        }
        if let Some(next) = self.staged.take() {
            self.live = next;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incompatible_layout_keeps_the_live_shader() {
        let mut reload = ReloadTransaction::new(1);
        reload.stage(2);
        assert!(!reload.commit(true, false));
        assert_eq!(*reload.live(), 1);
        assert!(reload.staged().is_none());
    }

    #[test]
    fn commit_waits_for_the_gpu_then_swaps() {
        let mut reload = ReloadTransaction::new(1);
        reload.stage(2);
        assert!(!reload.commit(false, true));
        assert_eq!(*reload.live(), 1);
        assert!(reload.commit(true, true));
        assert_eq!(*reload.live(), 2);
    }
}
