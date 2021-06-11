use vm::{gapi, module::{CLIENT_ID, Module, ModuleState}};
use vm_math::{CameraMatrices, Mat4f, OthroCameraTransforms, Transforms2D, Vec2f, create_2d_model_matrix, create_ortho_camera_matrices};

pub struct DebugServicesModule {
    frametime_text_mvp_matrix: Mat4f,
    screen_camera_matrices: CameraMatrices,
    screen_camera_transform: OthroCameraTransforms,
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

    fn update_frametime_text(&mut self) {
        let transforms = Transforms2D {
            position: Vec2f::new(10.0, self.screen_camera_transform.viewport_size.y - 24.),
            scaling: Vec2f::new(1., 1.),
            rotation: 0.,
        };

        let model_matrix = create_2d_model_matrix(transforms);
        self.frametime_text_mvp_matrix = self.screen_camera_matrices.mvp_matrix * model_matrix;
    }
}

impl Module for DebugServicesModule {
    fn id(&self) -> &'static str {
        "tech.paws.debug_services"
    }

    fn init(&mut self, _: &mut ModuleState) {}

    fn shutdown(&mut self, _: &mut ModuleState) {}

    fn step(&mut self, _: &mut ModuleState) {
        self.update_camera();
        self.update_frametime_text();
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
