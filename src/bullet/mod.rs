use bullet::bullets::Bullets;
use bullet::collision::Collision;
use cgmath::Point2;
use character::controls::CharacterInputState;
use game::constants::{ASPECT_RATIO, BULLET_SPEED, VIEW_DISTANCE};
use gfx;
use gfx_app::{ColorFormat, DepthFormat};
use graphics::{camera::CameraInputState, can_move, dimensions::{Dimensions, get_projection, get_view_matrix}};
use graphics::can_move_to_tile;
use graphics::mesh::PlainMesh;
use shaders::{bullet_pipeline, Position, Projection};
use specs;
use specs::prelude::{Read, ReadStorage, WriteStorage};
use std::f32;

pub mod bullets;
pub mod collision;

const SHADER_VERT: &[u8] = include_bytes!("../shaders/bullet.v.glsl");
const SHADER_FRAG: &[u8] = include_bytes!("../shaders/bullet.f.glsl");

#[derive(Debug, Clone, PartialEq)]
pub struct BulletDrawable {
  projection: Projection,
  pub position: Position,
  previous_position: Position,
  offset_delta: Position,
  pub movement_direction: Point2<f32>,
  pub status: collision::Collision,
}

impl BulletDrawable {
  pub fn new(position: Position, movement_direction: Point2<f32>) -> BulletDrawable {
    let view = get_view_matrix(VIEW_DISTANCE);
    let projection = get_projection(view, ASPECT_RATIO);
    BulletDrawable {
      projection,
      position,
      previous_position: Position::new(0.0, 0.0),
      offset_delta: Position::new(0.0, 0.0),
      movement_direction,
      status: Collision::Flying,
    }
  }

  pub fn update(&mut self, world_to_clip: &Projection, ci: &CharacterInputState) {
    self.projection = *world_to_clip;

    self.offset_delta =
      if (ci.movement.x() - self.previous_position.x()).abs() > f32::EPSILON ||
        (ci.movement.y() - self.previous_position.y()).abs() > f32::EPSILON {
        ci.movement - self.previous_position
      } else {
        self.offset_delta
      };

    self.previous_position = Position::new(
      ci.movement.x() - (self.movement_direction.x * BULLET_SPEED / (5.0 / 3.0)),
      ci.movement.y() + (self.movement_direction.y * BULLET_SPEED));

    self.position = self.position + self.offset_delta +
      Position::new(self.movement_direction.x * BULLET_SPEED / (5.0 / 3.0), -self.movement_direction.y * BULLET_SPEED);

    let tile_pos = ci.movement - self.position;

    self.status = if !can_move(self.position) {
      Collision::OutOfBounds
    } else if !can_move_to_tile(tile_pos) {
      Collision::Hit
    } else {
      Collision::Flying
    }
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

    let mesh = PlainMesh::new_with_data(factory, Point2::new(2.0, 2.0));

    let pso =
      match factory.create_pipeline_simple(SHADER_VERT, SHADER_FRAG, bullet_pipeline::new()) {
        Ok(val) => val,
        Err(err) => panic!("Bullet shader loading error {:?}", err)
      };

    let pipeline_data = bullet_pipeline::Data {
      vbuf: mesh.vertex_buffer,
      projection_cb: factory.create_constant_buffer(1),
      position_cb: factory.create_constant_buffer(1),
      out_color: rtv,
      out_depth: dsv,
    };

    BulletDrawSystem {
      bundle: gfx::Bundle::new(mesh.slice, pso, pipeline_data),
    }
  }

  pub fn draw<C>(&mut self,
                 drawable: &BulletDrawable,
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

impl<'a> specs::prelude::System<'a> for PreDrawSystem {
  type SystemData = (ReadStorage<'a, CameraInputState>,
                     WriteStorage<'a, Bullets>,
                     ReadStorage<'a, CharacterInputState>,
                     Read<'a, Dimensions>);

  fn run(&mut self, (camera_input, mut bullets, character_input, dim): Self::SystemData) {
    use specs::join::Join;

    for (camera, bs, ci) in (&camera_input, &mut bullets, &character_input).join() {
      let world_to_clip = dim.world_to_projection(camera);

      for b in &mut bs.bullets {
        b.update(&world_to_clip, ci);
      }
    }
  }
}
