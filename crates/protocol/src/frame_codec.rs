use mg_core::{Frame, GameFrame, Input, Slot, Tick};

use crate::proto;
use crate::wire_error::WireError;
use crate::wire_game::WireGame;

pub fn frame_to_wire<G: WireGame>(frame: &GameFrame<G>) -> proto::TickFrame {
    proto::TickFrame {
        tick: frame.tick.get(),
        inputs: frame.inputs.iter().map(input_to_wire::<G>).collect(),
        state_hash: frame.hash,
    }
}

fn input_to_wire<G: WireGame>(input: &Input<G::Command, G::WorldEvent>) -> proto::FrameInput {
    let input = match input {
        Input::Command { slot, command } => {
            proto::frame_input::Input::Command(proto::SlotCommand {
                slot: u32::from(slot.get()),
                command: Some(G::command_to_wire(command)),
            })
        }
        Input::World(event) => proto::frame_input::Input::World(G::world_event_to_wire(event)),
        Input::Dropped { slot } => proto::frame_input::Input::DroppedSlot(u32::from(slot.get())),
    };
    proto::FrameInput { input: Some(input) }
}

pub fn frame_from_wire<G: WireGame>(frame: &proto::TickFrame) -> Result<GameFrame<G>, WireError> {
    let inputs = frame
        .inputs
        .iter()
        .map(input_from_wire::<G>)
        .collect::<Result<_, _>>()?;
    Ok(Frame {
        tick: Tick::new(frame.tick),
        inputs,
        hash: frame.state_hash,
    })
}

fn input_from_wire<G: WireGame>(
    input: &proto::FrameInput,
) -> Result<Input<G::Command, G::WorldEvent>, WireError> {
    match input.input.as_ref().ok_or(WireError::Missing("input"))? {
        proto::frame_input::Input::Command(slot_command) => Ok(Input::Command {
            slot: slot_from_wire(slot_command.slot)?,
            command: G::command_from_wire(
                slot_command
                    .command
                    .as_ref()
                    .ok_or(WireError::Missing("command"))?,
            )?,
        }),
        proto::frame_input::Input::World(event) => {
            Ok(Input::World(G::world_event_from_wire(event)?))
        }
        proto::frame_input::Input::DroppedSlot(slot) => Ok(Input::Dropped {
            slot: slot_from_wire(*slot)?,
        }),
    }
}

pub fn slot_from_wire(slot: u32) -> Result<Slot, WireError> {
    u8::try_from(slot)
        .map(Slot::new)
        .map_err(|_| WireError::OutOfRange("slot"))
}
