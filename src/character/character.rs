use specs;

#[derive(Debug)]
pub struct Character;

impl specs::Component for Character {
  type Storage = specs::HashMapStorage<Character>;
}

#[derive(Debug)]
pub struct CharacterSprite {
  pub character_idx: usize,
  pub character_fire_idx: usize,
}

impl CharacterSprite {
  pub fn new() -> CharacterSprite {
    CharacterSprite {
      character_idx: 0,
      character_fire_idx: 0,
    }
  }

  pub fn update_run(&mut self) {
    if self.character_idx < 13 {
      self.character_idx += 1;
    } else {
      self.character_idx = 0;
    }
    self.character_fire_idx = 0;
  }

  pub fn update_fire(&mut self) {
    if self.character_fire_idx < 4 {
      self.character_fire_idx += 1;
    } else {
      self.character_fire_idx = 0;
    }
  }
}

impl specs::Component for CharacterSprite {
  type Storage = specs::VecStorage<CharacterSprite>;
}

#[derive(Debug)]
pub struct CharacterData {
  pub data: [f32; 4]
}

impl CharacterData {
  pub fn new(data: [f32; 4]) -> CharacterData {
    CharacterData { data }
  }
}
