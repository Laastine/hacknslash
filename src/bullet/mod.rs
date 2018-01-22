use bullet::bullets::Bullets;
use cgmath;
use cgmath::Point2;
use character::controls::CharacterInputState;
use game::constants::{ASPECT_RATIO, BULLET_SPEED};
use gfx;
use gfx_app::{ColorFormat, DepthFormat};
use graphics::Dimensions;
use graphics::camera::CameraInputState;
use shaders::{bullet_pipeline, VertexData, Position, Projection};
use specs;
use specs::{Fetch, ReadStorage, WriteStorage};
use std;

pub mod bullets;

const SHADER_VERT: &[u8] = include_bytes!("../shaders/bullet.v.glsl");
const SHADER_FRAG: &[u8] = include_bytes!("../shaders/bullet.f.glsl");

#[derive(Debug, Clone)]
pub struct BulletDrawable {
  projection: Projection,
  pub position: Position,
  previous_position: Position,
  offset_delta: Position,
  pub movement_direction: Point2<f32>,
  pub lifetime: usize,
}

impl BulletDrawable {
  pub fn new(position: cgmath::Point2<f32>, movement_direction: Point2<f32>) -> BulletDrawable {
    let view = Dimensions::get_view_matrix();
    BulletDrawable {
      projection: Projection {
        model: view.into(),
        view: view.into(),
        proj: cgmath::perspective(cgmath::Deg(75.0f32), ASPECT_RATIO, 0.1, 4000.0).into(),
      },
      position: Position {
        position: [position.x, position.y + 15.0],
      },
      previous_position: Position {
        position: [0.0, 0.0],
      },
      offset_delta: Position {
        position: [0.0, 0.0],
      },
      movement_direction,
      lifetime: 0,
    }
  }

  pub fn update(&mut self, world_to_clip: &Projection, ci: &CharacterInputState) {
    self.projection = *world_to_clip;

    self.offset_delta =
      if (ci.x_movement - self.previous_position.position[0]).abs() > std::f32::EPSILON ||
        (ci.y_movement - self.previous_position.position[1]).abs() > std::f32::EPSILON {
        Position {
          position: [ci.x_movement - self.previous_position.position[0], ci.y_movement - self.previous_position.position[1]]
        }
      } else {
        Position {
          position: [self.offset_delta.position[0], self.offset_delta.position[1]]
        }
      };

    self.previous_position = Position {
      position: [
        ci.x_movement - (self.movement_direction.x * BULLET_SPEED),
        ci.y_movement + (self.movement_direction.y * BULLET_SPEED)],
    };

    self.position =
      Position {
        position: [
          self.position.position[0] + self.offset_delta.position[0] + (self.movement_direction.x * BULLET_SPEED),
          self.position.position[1] + self.offset_delta.position[1] - (self.movement_direction.y * BULLET_SPEED)
        ]
      };

    self.lifetime += 1;
  }
}

pub struct BulletDrawSystem<R: gfx::Resources> {
  bundle: gfx::pso::bundle::Bundle<R, bullet_pipeline::Data<R>>,
}

impl<R: gfx::Resources> BulletDrawSystem<R> {
  pub fn new<F>(factory: &mut F,
                rtv: gfx::handle::RenderTargetView<R, ColorFormat>,
                dsv: gfx::handle::DepthStencilView<R, DepthFormat>) -> BulletDrawSystem<R>
    where F: gfx::Factory<R> {
    use gfx::traits::FactoryExt;

    let vertex_data: Vec<VertexData> =
      vec![
        VertexData::new([-1.0, -1.0, 0.0], [0.0, 1.0]),
        VertexData::new([1.0, -1.0, 0.0], [1.0, 1.0]),
        VertexData::new([1.0, 1.0, 0.0], [1.0, 0.0]),
        VertexData::new([-1.0, -1.0, 0.0], [0.0, 1.0]),
        VertexData::new([1.0, 1.0, 0.0], [1.0, 0.0]),
        VertexData::new([-1.0, 1.0, 0.0], [0.0, 0.0]),
      ];

    let (vertex_buf, slice) = factory.create_vertex_buffer_with_slice(&vertex_data, ());
    let pso = factory
      .create_pipeline_simple(SHADER_VERT,
                              SHADER_FRAG,
                              bullet_pipeline::new())
      .unwrap();

    let pipeline_data = bullet_pipeline::Data {
      vbuf: vertex_buf,
      projection_cb: factory.create_constant_buffer(1),
      position_cb: factory.create_constant_buffer(1),
      out_color: rtv,
      out_depth: dsv,
    };

    BulletDrawSystem {
      bundle: gfx::Bundle::new(slice, pso, pipeline_data),
    }
  }

  pub fn draw<C>(&mut self,
                 drawable: &mut BulletDrawable,
                 encoder: &mut gfx::Encoder<R, C>)
    where C: gfx::CommandBuffer<R> {
    encoder.update_constant_buffer(&self.bundle.data.projection_cb, &drawable.projection);
    encoder.update_constant_buffer(&self.bundle.data.position_cb, &drawable.position);
    self.bundle.encode(encoder);
  }
}

#[derive(Debug)]
pub struct PreDrawSystem;

impl PreDrawSystem {
  pub fn new() -> PreDrawSystem {
    PreDrawSystem {}
  }
}

impl<'a> specs::System<'a> for PreDrawSystem {
  type SystemData = (ReadStorage<'a, CameraInputState>,
                     WriteStorage<'a, Bullets>,
                     ReadStorage<'a, CharacterInputState>,
                     Fetch<'a, Dimensions>);

  fn run(&mut self, (camera_input, mut bullets, character_input, dim): Self::SystemData) {
    use specs::Join;

    for (camera, bs, ci) in (&camera_input, &mut bullets, &character_input).join() {
      let world_to_clip = dim.world_to_projection(camera);

      for b in bs.bullets.iter_mut() {
        b.update(&world_to_clip, ci);
      }
    }
  }
}
