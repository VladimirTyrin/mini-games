use mg_core::{Slot, Viewer};
use mg_protocol::proto;
use mg_registry::{CreateError, DynMirror};

use crate::registry_mirror::RegistryMirror;
use crate::strategy::Strategy;
use crate::view_strategy::ViewStrategy;

/// Plays any game with a [`ViewStrategy`].
#[derive(Debug)]
pub struct ViewBot<S: ViewStrategy>(pub S);

impl<S: ViewStrategy> Strategy for ViewBot<S> {
    type Mirror = RegistryMirror;

    fn create_mirror(
        &mut self,
        header: &proto::MatchHeader,
    ) -> Result<RegistryMirror, CreateError> {
        RegistryMirror::new(header)
    }

    fn decide(&mut self, mirror: &RegistryMirror, me: Slot) -> Option<proto::GameCommand> {
        self.0.decide(&mirror.view(Viewer::Player(me)), me)
    }
}
