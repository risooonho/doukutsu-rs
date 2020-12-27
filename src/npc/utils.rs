///! Various utility functions for NPC-related objects

use num_traits::abs;

use crate::bullet::Bullet;
use crate::caret::CaretType;
use crate::common::{Condition, Direction, Flag, Rect};
use crate::map::NPCData;
use crate::npc::{NPC, NPCFlag, NPCTable};
use crate::npc::list::NPCList;
use crate::player::Player;
use crate::rng::{RNG, Xoroshiro32PlusPlus};
use crate::shared_game_state::SharedGameState;

impl NPC {
    /// Initializes the RNG. Called when the [NPC] is being added to an [NPCList].
    pub(crate) fn init_rng(&mut self) {
        self.rng = Xoroshiro32PlusPlus::new((self.id as u32)
            .wrapping_sub(self.npc_type as u32)
            .wrapping_add(self.flag_num as u32)
            .wrapping_mul(214013)
            .wrapping_add(2531011) >> 5);
    }

    /// Creates a new NPC object with properties that have been populated with data from given NPC data table.
    pub fn create(npc_type: u16, table: &NPCTable) -> NPC {
        let display_bounds = table.get_display_bounds(npc_type);
        let hit_bounds = table.get_hit_bounds(npc_type);
        let (size, life, damage, flags, exp) = match table.get_entry(npc_type) {
            Some(entry) => { (entry.size, entry.life, entry.damage as u16, entry.npc_flags, entry.experience as u16) }
            None => { (2, 0, 0, NPCFlag(0), 0) }
        };
        let npc_flags = NPCFlag(flags.0);

        NPC {
            id: 0,
            npc_type,
            x: 0,
            y: 0,
            vel_x: 0,
            vel_y: 0,
            vel_x2: 0,
            vel_y2: 0,
            target_x: 0,
            target_y: 0,
            prev_x: 0,
            prev_y: 0,
            action_num: 0,
            anim_num: 0,
            flag_num: 0,
            event_num: 0,
            shock: 0,
            exp,
            size,
            life,
            damage,
            cond: Condition(0x00),
            flags: Flag(0),
            direction: if npc_flags.spawn_facing_right() { Direction::Right } else { Direction::Left },
            tsc_direction: 0,
            npc_flags,
            display_bounds,
            hit_bounds,
            parent_id: 0,
            action_counter: 0,
            action_counter2: 0,
            anim_counter: 0,
            anim_rect: Rect::new(0, 0, 0, 0),
            rng: Xoroshiro32PlusPlus::new(0),
        }
    }

    pub fn create_from_data(data: &NPCData, table: &NPCTable) -> NPC {
        let display_bounds = table.get_display_bounds(data.npc_type);
        let hit_bounds = table.get_hit_bounds(data.npc_type);
        let (size, life, damage, flags, exp) = match table.get_entry(data.npc_type) {
            Some(entry) => { (entry.size, entry.life, entry.damage as u16, entry.npc_flags, entry.experience as u16) }
            None => { (1, 0, 0, NPCFlag(0), 0) }
        };
        let npc_flags = NPCFlag(data.flags | flags.0);

        NPC {
            id: data.id,
            npc_type: data.npc_type,
            x: data.x as isize * 16 * 0x200,
            y: data.y as isize * 16 * 0x200,
            vel_x: 0,
            vel_y: 0,
            vel_x2: 0,
            vel_y2: 0,
            target_x: 0,
            target_y: 0,
            prev_x: 0,
            prev_y: 0,
            action_num: 0,
            anim_num: 0,
            flag_num: data.flag_num,
            event_num: data.event_num,
            shock: 0,
            exp,
            size,
            life,
            damage,
            cond: Condition(0x00),
            flags: Flag(0),
            direction: if npc_flags.spawn_facing_right() { Direction::Right } else { Direction::Left },
            tsc_direction: 0,
            npc_flags,
            display_bounds,
            hit_bounds,
            parent_id: 0,
            action_counter: 0,
            action_counter2: 0,
            anim_counter: 0,
            anim_rect: Rect::new(0, 0, 0, 0),
            rng: Xoroshiro32PlusPlus::new(0),
        }
    }

    /// Returns a reference to parent NPC (if present).
    pub fn get_parent_ref_mut<'a: 'b, 'b>(&self, npc_list: &'a NPCList) -> Option<&'b mut NPC> {
        match self.parent_id {
            0 => None,
            id if id == self.id => None,
            id => npc_list.get_npc(id as usize),
        }
    }

    /// Cycles animation frames in given range and speed.
    pub fn animate(&mut self, ticks_between_frames: u16, start_frame: u16, end_frame: u16) {
        self.anim_counter += 1;
        if self.anim_counter > ticks_between_frames {
            self.anim_counter = 0;
            self.anim_num += 1;
            if self.anim_num > end_frame {
                self.anim_num = start_frame;
            }
        }
    }

    /// Returns index of player that's closest to the current NPC.
    pub fn get_closest_player_idx_mut<'a>(&self, players: &[&'a mut Player; 2]) -> usize {
        let mut max_dist = f64::MAX;
        let mut player_idx = 0;

        for (idx, player) in players.iter().enumerate() {
            if !player.cond.alive() || player.cond.hidden() {
                continue;
            }

            let dist_x = abs(self.x - player.x) as f64;
            let dist_y = abs(self.y - player.y) as f64;
            let dist = (dist_x * dist_x + dist_y * dist_y).sqrt();

            if dist < max_dist {
                max_dist = dist;
                player_idx = idx;
            }
        }

        player_idx
    }

    /// Returns a reference to closest player.
    pub fn get_closest_player_mut<'a>(&self, players: [&'a mut Player; 2]) -> &'a mut Player {
        let idx = self.get_closest_player_idx_mut(&players);

        players[idx]
    }

    /// Returns true if the [NPC] collides with a [Bullet].
    pub fn collides_with_bullet(&self, bullet: &Bullet) -> bool {
        (
            self.npc_flags.shootable()
                && (self.x - self.hit_bounds.right as isize) < (bullet.x + bullet.enemy_hit_width as isize)
                && (self.x + self.hit_bounds.right as isize) > (bullet.x - bullet.enemy_hit_width as isize)
                && (self.y - self.hit_bounds.top as isize) < (bullet.y + bullet.enemy_hit_height as isize)
                && (self.y + self.hit_bounds.bottom as isize) > (bullet.y - bullet.enemy_hit_height as isize)
        ) || (
            self.npc_flags.invulnerable()
                && (self.x - self.hit_bounds.right as isize) < (bullet.x + bullet.hit_bounds.right as isize)
                && (self.x + self.hit_bounds.right as isize) > (bullet.x - bullet.hit_bounds.left as isize)
                && (self.y - self.hit_bounds.top as isize) < (bullet.y + bullet.hit_bounds.bottom as isize)
                && (self.y + self.hit_bounds.bottom as isize) > (bullet.y - bullet.hit_bounds.top as isize)
        )
    }

    /// Creates experience drop for this NPC.
    pub fn create_xp_drop(&self, state: &SharedGameState, npc_list: &NPCList) {
        let mut exp = self.exp;

        let mut xp_npc = NPC::create(1, &state.npc_table);
        xp_npc.cond.set_alive(true);
        xp_npc.direction = Direction::Left;
        xp_npc.x = self.x;
        xp_npc.y = self.y;

        while exp > 0 {
            let exp_piece = if exp >= 20 {
                exp -= 20;
                20
            } else if exp >= 5 {
                exp -= 5;
                5
            } else {
                exp -= 1;
                1
            };

            xp_npc.exp = exp_piece;

            let _ = npc_list.spawn(0x100, xp_npc.clone());
        }
    }

    /// Makes the NPC disappear and turns it into damage value holder.
    pub fn vanish(&mut self, state: &SharedGameState) {
        let mut npc = NPC::create(3, &state.npc_table);
        npc.cond.set_alive(true);
        npc.x = self.x;
        npc.y = self.y;

        *self = npc;
    }
}

#[allow(dead_code)]
impl NPCList {
    /// Returns true if at least one NPC with specified type is alive.
    #[inline]
    pub fn is_alive_by_type(&self, npc_type: u16) -> bool {
        self.iter_alive().any(|npc| npc.npc_type == npc_type)
    }

    /// Returns true if at least one NPC with specified event is alive.
    #[inline]
    pub fn is_alive_by_event(&self, event_num: u16) -> bool {
        self.iter_alive().any(|npc| npc.event_num == event_num)
    }

    /// Called once NPC is killed, creates smoke and drops.
    pub fn kill_npc(&self, id: usize, vanish: bool, can_drop_missile: bool, state: &mut SharedGameState) {
        if let Some(npc) = self.get_npc(id) {
            if let Some(table_entry) = state.npc_table.get_entry(npc.npc_type) {
                state.sound_manager.play_sfx(table_entry.death_sound);
            }

            match npc.size {
                1 => { self.create_death_smoke(npc.x, npc.y, npc.display_bounds.right, 3, state, &npc.rng); }
                2 => { self.create_death_smoke(npc.x, npc.y, npc.display_bounds.right, 7, state, &npc.rng); }
                3 => { self.create_death_smoke(npc.x, npc.y, npc.display_bounds.right, 12, state, &npc.rng); }
                _ => {}
            };

            if npc.exp != 0 {
                let rng = npc.rng.range(0..4);
                match rng {
                    0 => {
                        let mut heart_pick = NPC::create(87, &state.npc_table);
                        heart_pick.cond.set_alive(true);
                        heart_pick.direction = Direction::Left;
                        heart_pick.x = npc.x;
                        heart_pick.y = npc.y;
                        heart_pick.exp = if npc.exp > 6 { 6 } else { 2 };

                        let _ = self.spawn(0x100, heart_pick);
                    }
                    1 if can_drop_missile => {
                        let mut missile_pick = NPC::create(86, &state.npc_table);
                        missile_pick.cond.set_alive(true);
                        missile_pick.direction = Direction::Left;
                        missile_pick.x = npc.x;
                        missile_pick.y = npc.y;
                        missile_pick.exp = if npc.exp > 6 { 3 } else { 1 };

                        let _ = self.spawn(0x100, missile_pick);
                    }
                    _ => {
                        npc.create_xp_drop(state, self);
                    }
                }
            }

            state.game_flags.set(npc.flag_num as usize, true);
            
            if npc.npc_flags.show_damage() {
                // todo show damage
                if vanish {
                    npc.vanish(state);
                }
            } else {
                npc.cond.set_alive(false);
            }
        }
    }

    /// Removes NPCs whose event number matches the provided one.
    pub fn remove_by_event(&mut self, event_num: u16, state: &mut SharedGameState) {
        for npc in self.iter_alive() {
            if npc.event_num == event_num {
                npc.cond.set_alive(false);
                state.game_flags.set(npc.flag_num as usize, true);
            }
        }
    }

    /// Removes NPCs (and creates a smoke effect) whose type IDs match the provided one.
    pub fn remove_by_type(&mut self, npc_type: u16, state: &mut SharedGameState) {
        for npc in self.iter_alive() {
            if npc.npc_type == npc_type {
                npc.cond.set_alive(false);
                state.game_flags.set(npc.flag_num as usize, true);

                match npc.size {
                    1 => self.create_death_smoke(npc.x, npc.y, npc.display_bounds.right, 3, state, &npc.rng),
                    2 => self.create_death_smoke(npc.x, npc.y, npc.display_bounds.right, 7, state, &npc.rng),
                    3 => self.create_death_smoke(npc.x, npc.y, npc.display_bounds.right, 12, state, &npc.rng),
                    _ => {}
                };
            }
        }
    }

    /// Creates NPC death smoke diffusing in random directions.
    #[inline]
    pub fn create_death_smoke(&self, x: isize, y: isize, radius: usize, amount: usize, state: &mut SharedGameState, rng: &dyn RNG) {
        self.create_death_smoke_common(x, y, radius, amount, Direction::Left, state, rng)
    }

    /// Creates NPC death smoke diffusing upwards.
    #[inline]
    pub fn create_death_smoke_up(&self, x: isize, y: isize, radius: usize, amount: usize, state: &mut SharedGameState, rng: &dyn RNG) {
        self.create_death_smoke_common(x, y, radius, amount, Direction::Up, state, rng)
    }

    #[allow(clippy::too_many_arguments)]
    fn create_death_smoke_common(&self, x: isize, y: isize, radius: usize, amount: usize, direction: Direction, state: &mut SharedGameState, rng: &dyn RNG) {
        let radius = (radius / 0x200) as i32;

        let mut npc = NPC::create(4, &state.npc_table);
        npc.cond.set_alive(true);
        npc.direction = direction;

        for _ in 0..amount {
            let off_x = rng.range(-radius..radius) as isize * 0x200;
            let off_y = rng.range(-radius..radius) as isize * 0x200;

            npc.x = x + off_x;
            npc.y = y + off_y;

            let _ = self.spawn(0x100, npc.clone());
        }

        state.create_caret(x, y, CaretType::Explosion, Direction::Left);
    }
}
