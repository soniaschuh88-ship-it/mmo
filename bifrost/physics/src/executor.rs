//! PhysicsExecutor — applies `VoxelInstruction`s to `PhysicsWorld`.
//!
//! This is the core of the deterministic WASM physics kernel.
//! Every peer runs identical logic on identical input to produce
//! an identical `state_hash`. Divergence triggers a witness contest.
//!
//! # Determinism constraints
//!
//! - All loops iterate BTreeMap / sorted ranges — no HashMap, no arbitrary order.
//! - No SystemTime, no OS RNG, no platform-specific intrinsics.
//! - Radius checks use integer squared distances (no sqrt).
//! - Velocity is stored as f64 IEEE 754 bits — deterministic across WASM targets.

use bifrost_vis::{
    DamageFieldPayload, FillBoxPayload, InstructionPayload, MarchMaterialPayload,
    SetVoxelPayload, SimDebrisPayload, SimExplosionPayload, SimFirePayload,
    SimWaterPayload, SphereCutPayload, VoxelInstruction, VoxelProgram,
};

use crate::material::MaterialProps;
use crate::vec3::PhysicsVec3;
use crate::voxel::{flags, VoxelState};
use crate::world::PhysicsWorld;

/// Result of executing a `VoxelProgram` for one tick.
#[derive(Debug, Clone)]
pub struct PhysicsTickResult {
    pub tick:       u64,
    /// BLAKE3 hash of the world state after all instructions were applied.
    pub state_hash: [u8; 32],
    /// Number of instructions executed.
    pub instr_count: usize,
}

/// Stateless executor that applies `VoxelInstruction`s to a `PhysicsWorld`.
pub struct PhysicsExecutor;

impl PhysicsExecutor {
    /// Execute all instructions in `program` against `world`, advance the tick,
    /// and return the resulting state hash.
    pub fn execute_program(world: &mut PhysicsWorld, program: &VoxelProgram) -> PhysicsTickResult {
        for instr in &program.instructions {
            Self::execute_instruction(world, instr);
        }
        world.advance_tick();
        PhysicsTickResult {
            tick:        world.tick(),
            state_hash:  world.state_hash(),
            instr_count: program.instructions.len(),
        }
    }

    /// Execute a single instruction.
    pub fn execute_instruction(world: &mut PhysicsWorld, instr: &VoxelInstruction) {
        match &instr.payload {
            InstructionPayload::SetVoxel(p)      => Self::exec_set_voxel(world, p),
            InstructionPayload::FillBox(p)       => Self::exec_fill_box(world, p),
            InstructionPayload::SphereCut(p)     => Self::exec_sphere_cut(world, p),
            InstructionPayload::MarchMaterial(p) => Self::exec_march_material(world, p),
            InstructionPayload::DamageField(p)   => Self::exec_damage_field(world, p),
            InstructionPayload::SimWater(p)      => Self::exec_sim_water(world, p),
            InstructionPayload::SimFire(p)       => Self::exec_sim_fire(world, p),
            InstructionPayload::SimDebris(p)     => Self::exec_sim_debris(world, p),
            InstructionPayload::SimExplosion(p)  => Self::exec_sim_explosion(world, p),
        }
    }

    // ─── Opcode implementations ───────────────────────────────────────────────

    fn exec_set_voxel(world: &mut PhysicsWorld, p: &SetVoxelPayload) {
        let key = (p.position.x, p.position.y, p.position.z);
        world.set(key, VoxelState::solid(p.material));
    }

    fn exec_fill_box(world: &mut PhysicsWorld, p: &FillBoxPayload) {
        // Clamp iteration to sane bounds (prevent degenerate fill)
        let x0 = p.min.x.min(p.max.x);
        let x1 = p.min.x.max(p.max.x);
        let y0 = p.min.y.min(p.max.y);
        let y1 = p.min.y.max(p.max.y);
        let z0 = p.min.z.min(p.max.z);
        let z1 = p.min.z.max(p.max.z);
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    world.set((x, y, z), VoxelState::solid(p.material));
                }
            }
        }
    }

    fn exec_sphere_cut(world: &mut PhysicsWorld, p: &SphereCutPayload) {
        let r  = p.radius as i32;
        let cx = p.center.x;
        let cy = p.center.y;
        let cz = p.center.z;
        let r_sq = (p.radius as i64) * (p.radius as i64);
        for x in (cx - r)..=(cx + r) {
            for y in (cy - r)..=(cy + r) {
                for z in (cz - r)..=(cz + r) {
                    if PhysicsVec3::int_dist_sq(x, y, z, cx, cy, cz) <= r_sq {
                        world.set((x, y, z), VoxelState::solid(p.material));
                    }
                }
            }
        }
    }

    fn exec_march_material(world: &mut PhysicsWorld, p: &MarchMaterialPayload) {
        let mut x = p.origin.x;
        let mut y = p.origin.y;
        let mut z = p.origin.z;
        let dx = p.direction[0] as i32;
        let dy = p.direction[1] as i32;
        let dz = p.direction[2] as i32;
        for _ in 0..p.steps {
            world.set((x, y, z), VoxelState::solid(p.material));
            x += dx;
            y += dy;
            z += dz;
        }
    }

    fn exec_damage_field(world: &mut PhysicsWorld, p: &DamageFieldPayload) {
        let r    = p.radius as i32;
        let cx   = p.center.x;
        let cy   = p.center.y;
        let cz   = p.center.z;
        let r_sq = (p.radius as i64) * (p.radius as i64);

        // Collect keys first to avoid BTreeMap mutation during iteration
        let keys_in_range: Vec<(i32, i32, i32)> = {
            let mut keys = Vec::new();
            for x in (cx - r)..=(cx + r) {
                for y in (cy - r)..=(cy + r) {
                    for z in (cz - r)..=(cz + r) {
                        if PhysicsVec3::int_dist_sq(x, y, z, cx, cy, cz) <= r_sq {
                            keys.push((x, y, z));
                        }
                    }
                }
            }
            keys
        };

        for key in keys_in_range {
            if let Some(voxel) = world.get(key).cloned() {
                let mut v = voxel;
                v.apply_damage(p.damage);
                world.set(key, v);
            }
        }
    }

    fn exec_sim_water(world: &mut PhysicsWorld, p: &SimWaterPayload) {
        // Phase 1: simplified water — place water voxels downward from origin.
        // Full fluid simulation is Phase 2.
        let mut remaining = p.volume;
        let (ox, oy, oz) = (p.origin.x, p.origin.y, p.origin.z);
        let mut y = oy;
        while remaining > 0 {
            let key = (ox, y, oz);
            if world.get(key).is_none() {
                let mut voxel = VoxelState::solid(crate::material::MAT_WATER);
                voxel.flags |= flags::FLOODED;
                world.set(key, voxel);
                remaining = remaining.saturating_sub(1);
            }
            y -= 1;
            if y < oy - 64 { break; } // safety limit
        }
    }

    fn exec_sim_fire(world: &mut PhysicsWorld, p: &SimFirePayload) {
        // Phase 1: simplified fire — mark origin voxel as on-fire.
        // Full fire propagation is Phase 2 (cellular automaton).
        let key = (p.origin.x, p.origin.y, p.origin.z);
        let props = world.get(key)
            .map(|v| MaterialProps::for_material(v.material))
            .unwrap_or_else(|| MaterialProps::for_material(0));

        if props.flammable {
            if let Some(voxel) = world.get(key).cloned() {
                let mut v = voxel;
                v.flags |= flags::ON_FIRE;
                world.set(key, v);
            }
        }
        // Intensity and fuel are stored for future Phase 2 use
        let _ = (p.intensity, p.fuel);
    }

    fn exec_sim_debris(world: &mut PhysicsWorld, p: &SimDebrisPayload) {
        // Phase 1: simplified debris — mark nearby voxels as unstable.
        // Full debris particle simulation (with velocity) is Phase 2.
        let r = (p.count as f64).cbrt().ceil() as i32;
        let (ox, oy, oz) = (p.origin.x, p.origin.y, p.origin.z);
        let r_sq = (r as i64) * (r as i64);

        let keys: Vec<_> = {
            let mut v = Vec::new();
            for x in (ox - r)..=(ox + r) {
                for y in (oy - r)..=(oy + r) {
                    for z in (oz - r)..=(oz + r) {
                        if PhysicsVec3::int_dist_sq(x, y, z, ox, oy, oz) <= r_sq {
                            v.push((x, y, z));
                        }
                    }
                }
            }
            v
        };

        let impulse_vel = p.impulse as f64 / 1000.0;
        for key in keys {
            if let Some(voxel) = world.get(key).cloned() {
                let mut v = voxel;
                v.flags |= flags::UNSTABLE;
                v.velocity = PhysicsVec3::new(impulse_vel, impulse_vel, impulse_vel);
                world.set(key, v);
            }
        }
    }

    fn exec_sim_explosion(world: &mut PhysicsWorld, p: &SimExplosionPayload) {
        // Explosion = sphere_cut + damage_field + debris scatter
        // 1. Excavate a sphere
        let cut = SphereCutPayload {
            center: p.center,
            radius: p.radius,
            material: p.result_material,
        };
        Self::exec_sphere_cut(world, &cut);

        // 2. Apply damage field at 1.5× radius
        let damage_radius = (p.radius * 3 / 2).max(1);
        let damage_amount = (p.force / 100).min(u16::MAX as u32) as u16;
        let dmg = DamageFieldPayload {
            center: p.center,
            radius: damage_radius,
            damage: damage_amount,
        };
        Self::exec_damage_field(world, &dmg);

        // 3. Mark debris at 2× radius
        let debris = SimDebrisPayload {
            origin: p.center,
            count: p.radius * p.radius, // proportional to blast area
            impulse: (p.force / 10).min(u32::from(u16::MAX)) as u16,
        };
        Self::exec_sim_debris(world, &debris);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bifrost_vis::{
        FillBoxPayload, InstructionPayload, SetVoxelPayload, SimExplosionPayload,
        SphereCutPayload, VoxelCoord, VoxelProgram,
    };
    use crate::material::{MAT_AIR, MAT_STONE};

    fn stone_at(x: i32, y: i32, z: i32) -> (VoxelInstruction, InstructionPayload) {
        let p = InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(x, y, z),
            material: MAT_STONE,
        });
        (VoxelInstruction::new(1, p.clone()).unwrap(), p)
    }

    #[test]
    fn set_voxel_places_stone() {
        let mut world = PhysicsWorld::new();
        let (instr, _) = stone_at(5, 5, 5);
        PhysicsExecutor::execute_instruction(&mut world, &instr);
        assert_eq!(world.get((5, 5, 5)).unwrap().material, MAT_STONE);
    }

    #[test]
    fn fill_box_places_all_voxels() {
        let mut world = PhysicsWorld::new();
        let p = InstructionPayload::FillBox(FillBoxPayload {
            min: VoxelCoord::new(0, 0, 0),
            max: VoxelCoord::new(2, 2, 2),
            material: MAT_STONE,
        });
        let instr = VoxelInstruction::new(1, p).unwrap();
        PhysicsExecutor::execute_instruction(&mut world, &instr);
        // 3×3×3 = 27 voxels
        assert_eq!(world.voxel_count(), 27);
    }

    #[test]
    fn sphere_cut_places_voxels() {
        let mut world = PhysicsWorld::new();
        let p = InstructionPayload::SphereCut(SphereCutPayload {
            center: VoxelCoord::new(0, 0, 0),
            radius: 5,
            material: MAT_STONE,
        });
        let instr = VoxelInstruction::new(1, p).unwrap();
        PhysicsExecutor::execute_instruction(&mut world, &instr);
        assert!(world.voxel_count() > 0);
        // Verify a voxel exactly at center
        assert_eq!(world.get((0, 0, 0)).unwrap().material, MAT_STONE);
        // Verify a voxel outside radius is absent
        assert!(world.get((10, 10, 10)).is_none());
    }

    #[test]
    fn explosion_excavates_sphere() {
        let mut world = PhysicsWorld::new();
        // First fill a region with stone
        let fill = InstructionPayload::FillBox(FillBoxPayload {
            min: VoxelCoord::new(-20, -20, -20),
            max: VoxelCoord::new(20, 20, 20),
            material: MAT_STONE,
        });
        PhysicsExecutor::execute_instruction(&mut world, &VoxelInstruction::new(1, fill).unwrap());
        let before = world.voxel_count();

        // Explode
        let exp = InstructionPayload::SimExplosion(SimExplosionPayload {
            center: VoxelCoord::new(0, 0, 0),
            radius: 5,
            force: 1000,
            result_material: MAT_AIR,
        });
        PhysicsExecutor::execute_instruction(&mut world, &VoxelInstruction::new(1, exp).unwrap());
        let after = world.voxel_count();
        // Explosion should have removed voxels in the center
        assert!(after < before);
        // Center should now be air
        assert!(world.get((0, 0, 0)).is_none()); // air = absent
    }

    #[test]
    fn program_hash_same_across_identical_runs() {
        let build_program = || -> VoxelProgram {
            let mut p = VoxelProgram::new();
            p.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
                position: VoxelCoord::new(1, 2, 3),
                material: MAT_STONE,
            })).unwrap();
            p
        };

        let mut w1 = PhysicsWorld::new();
        let r1 = PhysicsExecutor::execute_program(&mut w1, &build_program());

        let mut w2 = PhysicsWorld::new();
        let r2 = PhysicsExecutor::execute_program(&mut w2, &build_program());

        assert_eq!(r1.state_hash, r2.state_hash,
            "Same program on same initial state must produce identical state hash");
    }

    #[test]
    fn different_programs_different_hashes() {
        let mut w1 = PhysicsWorld::new();
        let mut p1 = VoxelProgram::new();
        p1.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(0, 0, 0), material: MAT_STONE,
        })).unwrap();
        let r1 = PhysicsExecutor::execute_program(&mut w1, &p1);

        let mut w2 = PhysicsWorld::new();
        let mut p2 = VoxelProgram::new();
        p2.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(1, 0, 0), material: MAT_STONE, // different position!
        })).unwrap();
        let r2 = PhysicsExecutor::execute_program(&mut w2, &p2);

        assert_ne!(r1.state_hash, r2.state_hash);
    }
}
