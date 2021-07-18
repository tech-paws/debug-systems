pub mod commands;
pub mod profiler;

mod commands_registry;
mod state;

use profiler::{ProfileState, PROFILE_STATE};
use state::DEBUG_STATE;
use vm::{
    gapi,
    module::{Module, ModuleState, StepState, CLIENT_ID},
};
use vm_math::{
    create_2d_model_matrix, create_ortho_camera_matrices, CameraMatrices, Mat4f,
    OthroCameraTransforms, Transforms2D, Vec2f,
};

pub struct DebugServicesModule {
    frametime_text_mvp_matrix: Mat4f,
    screen_camera_matrices: CameraMatrices,
    screen_camera_transform: OthroCameraTransforms,
}

struct DebugContext<'a> {
    pos: Vec2f,
    _profile_state: &'a ProfileState,
}

impl Default for DebugServicesModule {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugServicesModule {
    pub fn new() -> Self {
        Self {
            frametime_text_mvp_matrix: Mat4f::IDENT,
            screen_camera_matrices: CameraMatrices::default(),
            screen_camera_transform: OthroCameraTransforms {
                viewport_size: Vec2f::new(1024., 768.),
                position: Vec2f::ZERO,
                zoom: 1.,
            },
        }
    }

    fn update_camera(&mut self) {
        self.screen_camera_matrices = create_ortho_camera_matrices(self.screen_camera_transform);
    }

    fn update_frametime_text(&mut self) -> Vec2f {
        let transforms = Transforms2D {
            position: Vec2f::new(10.0, self.screen_camera_transform.viewport_size.y - 24.),
            scaling: Vec2f::new(1., 1.),
            rotation: 0.,
        };

        let model_matrix = create_2d_model_matrix(transforms);
        self.frametime_text_mvp_matrix = self.screen_camera_matrices.mvp_matrix * model_matrix;

        Vec2f::new(0., 24.)
    }
}

impl Module for DebugServicesModule {
    fn id(&self) -> &'static str {
        "tech.paws.debug_services"
    }

    fn init(&mut self, _: &mut ModuleState) {
        let debug_state = &mut DEBUG_STATE.lock().expect("failed to get debug state");
        commands_registry::init(debug_state);
    }

    fn shutdown(&mut self, _: &mut ModuleState) {}

    fn step(&mut self, state: &mut ModuleState) -> StepState {
        for event in state.client_info.events.iter() {
            #[allow(clippy::single_match)]
            match event {
                vm::module::ClientEvent::WindowResize { w, h } => {
                    self.screen_camera_transform.viewport_size = Vec2f::new(*w, *h);
                }
                _ => {}
            }
        }

        let _debug_state = &mut DEBUG_STATE.lock().expect("failed to get debug state");
        let profile_state = &mut PROFILE_STATE.lock().expect("failed to get profile state");

        let mut context = DebugContext {
            pos: Vec2f::new(10.0, 10.0),
            _profile_state: profile_state,
        };

        self.update_camera();
        let size = self.update_frametime_text();

        context.pos.y += size.y;
        context.pos.x = 5.;

        StepState::None
    }

    fn render(&mut self, state: &mut ModuleState) {
        let context = gapi::GApiContext {
            from: self.id(),
            address: CLIENT_ID,
            commands_bus: &mut state.commands_bus,
        };

        let frame_time = format!("Frame Time: {:?}", state.last_time.elapsed());
        let text = gapi::TextData {
            font_id: 0,
            font_size: 14,
            mvp_matrix: self.frametime_text_mvp_matrix,
            text: frame_time,
        };

        gapi::draw_texts(&context, &[text]);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
