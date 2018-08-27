use bullet;
use character;
use critter::CharacterSprite;
use game::constants::{CURRENT_AMMO_TEXT, HUD_TEXTS, VERSION_NUMBER_TEXT};
use gfx;
use gfx_app::{ColorFormat, DepthFormat};
use gfx_app::renderer::EncoderQueue;
use graphics::{DeltaTime, orientation::Stance};
use graphics::Drawables;
use hud;
use specs;
use specs::prelude::{Read, WriteStorage};
use std::time::Instant;
use terrain;
use terrain_object;
use terrain_object::TerrainTexture;
use zombie;

pub struct DrawSystem<D: gfx::Device> {
  render_target_view: gfx::handle::RenderTargetView<D::Resources, ColorFormat>,
  depth_stencil_view: gfx::handle::DepthStencilView<D::Resources, DepthFormat>,
  terrain_system: terrain::TerrainDrawSystem<D::Resources>,
  character_system: character::CharacterDrawSystem<D::Resources>,
  zombie_system: zombie::ZombieDrawSystem<D::Resources>,
  bullet_system: bullet::BulletDrawSystem<D::Resources>,
  terrain_object_system: [terrain_object::TerrainObjectDrawSystem<D::Resources>; 3],
  text_system: [hud::TextDrawSystem<D::Resources>; 3],
  encoder_queue: EncoderQueue<D>,
  game_time: Instant,
  frames: u32,
  cool_down: f64,
  run_cool_down: f64,
  fire_cool_down: f64,
}

impl<D: gfx::Device> DrawSystem<D> {
  pub fn new<F>(factory: &mut F,
                rtv: &gfx::handle::RenderTargetView<D::Resources, ColorFormat>,
                dsv: &gfx::handle::DepthStencilView<D::Resources, DepthFormat>,
                encoder_queue: EncoderQueue<D>)
                -> DrawSystem<D>
                where F: gfx::Factory<D::Resources> {
    DrawSystem {
      render_target_view: rtv.clone(),
      depth_stencil_view: dsv.clone(),
      terrain_system: terrain::TerrainDrawSystem::new(factory, rtv.clone(), dsv.clone()),
      character_system: character::CharacterDrawSystem::new(factory, rtv.clone(), dsv.clone()),
      zombie_system: zombie::ZombieDrawSystem::new(factory, rtv.clone(), dsv.clone()),
      bullet_system: bullet::BulletDrawSystem::new(factory, rtv.clone(), dsv.clone()),
      terrain_object_system: [
        terrain_object::TerrainObjectDrawSystem::new(factory, rtv.clone(), dsv.clone(), TerrainTexture::Ammo),
        terrain_object::TerrainObjectDrawSystem::new(factory, rtv.clone(), dsv.clone(), TerrainTexture::House),
        terrain_object::TerrainObjectDrawSystem::new(factory, rtv.clone(), dsv.clone(), TerrainTexture::Tree)
      ],
      text_system: [
        hud::TextDrawSystem::new(factory, &HUD_TEXTS, VERSION_NUMBER_TEXT, rtv.clone(), dsv.clone()),
        hud::TextDrawSystem::new(factory, &HUD_TEXTS, CURRENT_AMMO_TEXT, rtv.clone(), dsv.clone()),
        hud::TextDrawSystem::new(factory, &HUD_TEXTS, CURRENT_AMMO_TEXT, rtv.clone(), dsv.clone())
      ],
      encoder_queue,
      game_time: Instant::now(),
      frames: 0,
      cool_down: 1.0,
      run_cool_down: 1.0,
      fire_cool_down: 1.0,
    }
  }
}

impl<'a, D> specs::prelude::System<'a> for DrawSystem<D>
  where D: gfx::Device,
        D::CommandBuffer: Send {
  #[cfg_attr(feature = "cargo-clippy", allow(type_complexity))]
  type SystemData = (WriteStorage<'a, terrain::TerrainDrawable>,
                     WriteStorage<'a, character::CharacterDrawable>,
                     WriteStorage<'a, CharacterSprite>,
                     WriteStorage<'a, hud::hud_objects::HudObjects>,
                     WriteStorage<'a, zombie::zombies::Zombies>,
                     WriteStorage<'a, bullet::bullets::Bullets>,
                     WriteStorage<'a, terrain_object::terrain_objects::TerrainObjects>,
                     Read<'a, DeltaTime>);

  fn run(&mut self, (mut terrain, mut character, mut character_sprite, mut hud_objects, mut zombies, mut bullets, mut terrain_objects, dt): Self::SystemData) {
    use specs::join::Join;
    let mut encoder = self.encoder_queue.receiver.recv().unwrap();

    let delta = dt.0;

    if self.cool_down == 0.0 {
      self.cool_down += 0.05;
    }
    if self.fire_cool_down == 0.0 {
      self.fire_cool_down += 0.2;
    }
    if self.run_cool_down == 0.0 {
      self.run_cool_down += 0.02;
    }
    self.cool_down = (self.cool_down - delta).max(0.0);
    self.run_cool_down = (self.run_cool_down - delta).max(0.0);
    self.fire_cool_down = (self.fire_cool_down - delta).max(0.0);

    let current_time = Instant::now();
    self.frames += 1;
    if cfg!(feature = "framerate") && (current_time.duration_since(self.game_time).as_secs()) >= 1 {
      println!("{:?} ms/frames", 1000.0 / f64::from(self.frames));
      self.frames = 0;
      self.game_time = Instant::now();
    }

    encoder.clear(&self.render_target_view, [16.0 / 256.0, 16.0 / 256.0, 20.0 / 256.0, 1.0]);
    encoder.clear_depth(&self.depth_stencil_view, 1.0);

    for (t, c, cs, hds, zs, bs, obj) in (&mut terrain, &mut character, &mut character_sprite, &mut hud_objects,
                                             &mut zombies, &mut bullets, &mut terrain_objects).join() {
      self.terrain_system.draw(t, &mut encoder);

      for hud in &mut hds.objects {
        self.text_system[0].draw(hud, &mut encoder);
        self.text_system[1].draw(hud, &mut encoder);
      }

      if self.cool_down == 0.0 {
        if c.stance == Stance::Walking {
          cs.update_run();
        }
        for mut z in &mut zs.zombies {
          match z.stance {
            Stance::NormalDeath => z.update_death_idx(5),
            Stance::CriticalDeath => z.update_death_idx(7),
            Stance::Walking => z.update_alive_idx(7),
            Stance::Still => z.update_alive_idx(3),
            _ => ()
          };
        }
      } else if self.fire_cool_down == 0.0 && c.stance == Stance::Firing {
        cs.update_fire();
      }

      if self.run_cool_down == 0.0 {
        for mut z in &mut zs.zombies {
          if let Stance::Running = z.stance {
             z.update_alive_idx(7)
          }
        }
      }

      let mut drawables: Vec<Drawables> = vec![];
      drawables.append(&mut bs.bullets.iter().map(|b| Drawables::Bullet(b)).collect());
      drawables.append(&mut zs.zombies.iter_mut().map(|z| Drawables::Zombie(z)).collect());

      for (idx, o) in obj.objects.iter().enumerate() {
        match idx {
          0 => drawables.push(Drawables::TerrainAmmo(o)),
          1 | 2 => drawables.push(Drawables::TerrainHouse(o)),
          _ => drawables.push(Drawables::TerrainTree(o)),
        };
      }

      drawables.push(Drawables::Character(c));

      drawables.sort_by(|a, b| {
        Drawables::get_y(b)
          .partial_cmp(&Drawables::get_y(a)).unwrap()
      });

      for mut e in &mut drawables {
        match *e {
          Drawables::Bullet(ref e) => { self.bullet_system.draw(e, &mut encoder) },
          Drawables::Zombie(ref mut e) => { self.zombie_system.draw(e, &mut encoder) },
          Drawables::TerrainAmmo(ref mut e) => { self.terrain_object_system[0].draw(e, &mut encoder) },
          Drawables::TerrainHouse(ref mut e) => { self.terrain_object_system[1].draw(e, &mut encoder) },
          Drawables::TerrainTree(ref mut e) => { self.terrain_object_system[2].draw(e, &mut encoder) },
          Drawables::Character(ref mut e) => { self.character_system.draw(e, cs, &mut encoder) },
        }
      }
    }

    self.encoder_queue.sender.send(encoder);
  }
}
