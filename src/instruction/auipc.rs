use crate::engine::Engine;
use crate::error::EmbiveError;
use crate::instruction::format::TypeU;
use crate::instruction::{Instruction, Opcode, INSTRUCTION_SIZE};

/// Add Upper Immediate to Program Counter
/// Both an Opcode and an Instruction
/// Format: U-Type.
/// Action: rd = PC + imm
pub struct Auipc {
    ty: TypeU,
}

impl Opcode for Auipc {
    #[inline(always)]
    fn decode(data: u32) -> impl Instruction {
        Self {
            ty: TypeU::from(data),
        }
    }
}

impl Instruction for Auipc {
    #[inline(always)]
    fn execute(&self, engine: &mut Engine) -> Result<bool, EmbiveError> {
        if self.ty.rd != 0 {
            // rd = 0 means its a HINT instruction, just ignore it.
            // Load the immediate value + pc into the register.
            let reg = engine.registers.get_mut(self.ty.rd)?;
            *reg = engine.program_counter.wrapping_add_signed(self.ty.imm) as i32;
        }

        // Go to next instruction
        engine.program_counter = engine.program_counter.wrapping_add(INSTRUCTION_SIZE);

        // Continue execution
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auipc() {
        let mut engine = Engine::new(&[], &mut [], Default::default()).unwrap();
        engine.program_counter = 0x1;
        let auipc = Auipc {
            ty: TypeU { rd: 1, imm: 0x1000 },
        };

        let result = auipc.execute(&mut engine);
        assert_eq!(result, Ok(true));
        assert_eq!(*engine.registers.get_mut(1).unwrap(), 0x1001);
        assert_eq!(engine.program_counter, 0x1 + INSTRUCTION_SIZE);
    }

    #[test]
    fn test_auipc_negative() {
        let mut engine = Engine::new(&[], &mut [], Default::default()).unwrap();
        engine.program_counter = 0x1;
        let auipc = Auipc {
            ty: TypeU {
                rd: 1,
                imm: -0x1000,
            },
        };

        let result = auipc.execute(&mut engine);
        assert_eq!(result, Ok(true));
        assert_eq!(*engine.registers.get_mut(1).unwrap(), -0xfff);
        assert_eq!(engine.program_counter, 0x1 + INSTRUCTION_SIZE);
    }
}
