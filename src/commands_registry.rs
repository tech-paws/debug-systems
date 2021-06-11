use std::sync::MutexGuard;

use crate::commands::*;
// use crate::profile;
use crate::state::DebugState;

pub fn init(debug_state: &mut MutexGuard<DebugState>) {
    register_command(
        debug_state,
        "Update snapshot interval",
        Command {
            namespace: String::from("profile"),
            name: String::from("set_snapshot_interval"),
            executor: set_snapshot_interval_command,
        },
    );
}

fn set_snapshot_interval_command(
    _: &mut MutexGuard<DebugState>,
    arguments: &[CommandArgument],
) -> Result<(), String> {
    require(arguments.len() == 1, "bad arguments length")?;

    let interval = match arguments[0] {
        CommandArgument::Number(val) => Ok(val),
        _ => Err(String::from("Argument should be int")),
    }?;

    // let mut profile_state = profile::get_profile_state();
    // profile::update_snapshot_interval(&mut profile_state, interval as usize);

    Ok(())
}
