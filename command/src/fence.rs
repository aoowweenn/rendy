use {
    crate::family::QueueId,
    gfx_hal::{Backend, Device},
};

/// Queue epoch is the point in particluar queue timeline when fence is submitted.
#[derive(Clone, Copy, Debug)]
pub struct FenceEpoch {
    /// Queue that signals fence.
    pub queue: QueueId,

    /// Queue epoch counter.
    pub epoch: u64,
}

#[derive(Clone, Copy, Debug)]
enum FenceState {
    Unsignaled,
    Signaled,
    Submitted(FenceEpoch),
}

/// Fence wrapper.
#[derive(Debug)]
pub struct Fence<B: Backend> {
    raw: B::Fence,
    state: FenceState,
}

impl<B> Fence<B>
where
    B: Backend,
{
    /// Create new fence in signaled or unsignaled state.
    pub fn new(
        device: &impl Device<B>,
        signaled: bool,
    ) -> Result<Self, gfx_hal::device::OutOfMemory> {
        let raw = device.create_fence(false)?;
        Ok(Fence {
            raw,
            state: if signaled {
                FenceState::Signaled
            } else {
                FenceState::Unsignaled
            },
        })
    }

    /// Check if fence was submitted.
    pub fn is_submitted(&self) -> bool {
        match self.state {
            FenceState::Submitted(_) => true,
            _ => false,
        }
    }

    /// Check if fence is signaled.
    pub fn is_signaled(&self) -> bool {
        match self.state {
            FenceState::Signaled => true,
            _ => false,
        }
    }

    /// Check if fence is unsignaled.
    /// It can be submitted as well.
    pub fn is_unsignaled(&self) -> bool {
        !self.is_signaled()
    }

    /// Panics if signaled or submitted.
    /// Becomes `Submitted` after.
    pub(crate) fn mark_submitted(&mut self, epoch: FenceEpoch) {
        match self.state {
            FenceState::Unsignaled => {
                self.state = FenceState::Submitted(epoch);
            }
            _ => panic!("Must be Unsignaled"),
        }
    }

    /// Reset signaled fence.
    /// Panics if not signaled.
    /// Becomes unsigneled.
    pub unsafe fn reset(
        &mut self,
        device: &impl Device<B>,
    ) -> Result<(), gfx_hal::device::OutOfMemory> {
        match self.state {
            FenceState::Signaled => {
                device.reset_fence(&self.raw)?;
                self.state = FenceState::Unsignaled;
                Ok(())
            }
            _ => panic!("Must be signaled"),
        }
    }

    /// Mark signaled fence as reset.
    /// Panics if not signaled.
    /// Becomes unsigneled.
    /// Fence must be reset using raw fence value.
    pub unsafe fn mark_reset(&mut self) {
        match self.state {
            FenceState::Signaled => {
                self.state = FenceState::Unsignaled;
            }
            _ => panic!("Must be signaled"),
        }
    }

    /// Mark fence as signaled.
    /// Panics if not submitted.
    /// Fence must be checked to be signaled using raw fence value.
    pub unsafe fn mark_signaled(&mut self) -> FenceEpoch {
        match self.state {
            FenceState::Submitted(epoch) => {
                self.state = FenceState::Signaled;
                epoch
            }
            _ => panic!("Must be submitted"),
        }
    }

    /// Wait for fence to become signaled.
    /// Panics if not submitted.
    /// Returns submission epoch on success.
    pub unsafe fn wait_signaled(
        &mut self,
        device: &impl Device<B>,
        timeout_ns: u64,
    ) -> Result<Option<FenceEpoch>, gfx_hal::device::OomOrDeviceLost> {
        match self.state {
            FenceState::Submitted(epoch) => {
                if device.wait_for_fence(&self.raw, timeout_ns)? {
                    self.state = FenceState::Signaled;
                    Ok(Some(epoch))
                } else {
                    Ok(None)
                }
            }
            _ => panic!("Must be submitted"),
        }
    }

    /// Check if fence has became signaled.
    /// Panics if not submitted.
    /// Returns submission epoch on success.
    pub unsafe fn check_signaled(
        &mut self,
        device: &impl Device<B>,
    ) -> Result<Option<FenceEpoch>, gfx_hal::device::DeviceLost> {
        match self.state {
            FenceState::Submitted(epoch) => {
                if device.get_fence_status(&self.raw)? {
                    self.state = FenceState::Signaled;
                    Ok(Some(epoch))
                } else {
                    Ok(None)
                }
            }
            _ => panic!("Must be submitted"),
        }
    }

    /// Get raw fence reference.
    /// Use `mark_*` functions to reflect stage changes.
    pub fn raw(&self) -> &B::Fence {
        &self.raw
    }

    /// Get submission epoch.
    /// Panics if not submitted.
    pub fn epoch(&self) -> FenceEpoch {
        match self.state {
            FenceState::Submitted(epoch) => epoch,
            _ => panic!("Must be submitted"),
        }
    }

    /// Unwrap raw fence value.
    /// Panics if submitted.
    pub fn into_inner(self) -> B::Fence {
        match self.state {
            FenceState::Signaled | FenceState::Unsignaled => self.raw,
            _ => panic!("Submitted fence must be waited upon before destroying"),
        }
    }
}
