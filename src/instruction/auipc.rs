use crate::engine::Engine;
use crate::error::EmbiveError;
use crate::instruction::format::TypeU;
use crate::instruction::{Instruction, INSTRUCTION_SIZE};
use crate::memory::Memory;

/// Add Upper Immediate to Program Counter
/// Both an Opcode and an Instruction
/// Format: U-Type.
/// Action: rd = PC + imm
pub struct Auipc {}

impl<M: Memory> Instruction<M> for Auipc {
    #[inline(always)]
    fn decode_execute(data: u32, engine: &mut Engine<M>) -> Result<bool, EmbiveError> {
        let inst = TypeU::from(data);

        if inst.rd != 0 {
            // rd = 0 means its a HINT instruction, just ignore it.
            // Load the immediate value + pc into the register.
            let reg = engine.registers.get_mut(inst.rd)?;
            *reg = engine.program_counter.wrapping_add_signed(inst.imm) as i32;
        }

        // Go to next instruction
        engine.program_counter = engine.program_counter.wrapping_add(INSTRUCTION_SIZE);

        // Continue execution
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::memory::SliceMemory;

    use super::*;

    #[test]
    fn test_auipc() {
        let mut memory = SliceMemory::new(&[], &mut []);
        let mut engine = Engine::new(&mut memory, Default::default()).unwrap();
        engine.program_counter = 0x1;
        let auipc = TypeU { rd: 1, imm: 0x1000 };

        let result = Auipc::decode_execute(auipc.into(), &mut engine);
        assert_eq!(result, Ok(true));
        assert_eq!(*engine.registers.get_mut(1).unwrap(), 0x1001);
        assert_eq!(engine.program_counter, 0x1 + INSTRUCTION_SIZE);
    }

    #[test]
    fn test_auipc_negative() {
        let mut memory = SliceMemory::new(&[], &mut []);
        let mut engine = Engine::new(&mut memory, Default::default()).unwrap();
        engine.program_counter = 0x1;
        let auipc = TypeU {
            rd: 1,
            imm: -0x1000,
        };

        let result = Auipc::decode_execute(auipc.into(), &mut engine);
        assert_eq!(result, Ok(true));
        assert_eq!(*engine.registers.get_mut(1).unwrap(), -0xfff);
        assert_eq!(engine.program_counter, 0x1 + INSTRUCTION_SIZE);
    }
}
