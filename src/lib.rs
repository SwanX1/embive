//! # Embive (Embedded RISC-V)
//!
//! Embive is a low-level sandboxing library focused on the embedding of untrusted code for constrained environments.  
//! As it interprets RISC-V bytecode, multiple languages are supported out of the box by Embive (Rust, C, C++, Zig, TinyGo, etc.).  
//! By default, it doesn’t require external crates, dynamic memory allocation or the standard library (`no_std` & `no_alloc`).
//!
//! Embive is designed for any error during execution to be recoverable, allowing the host to handle it as needed.
//! As so, no panics should occur on release builds, despite the bytecode being executed.
//!
//! Currently, it supports the `RV32I[M]` unprivileged instruction set (M extension enabled by default, check [Features](#features)).
//!
//! ## Bytecode
//! The bytecode can be generated by any compiler that supports the RV32I\[M\] instruction set, as long as it can output a flat binary file
//! (`.bin`) statically linked to the correct addresses (Code at `0x00000000`, RAM at [`crate::memory::RAM_OFFSET`]).
//!
//! ## Example
//!
//! ```
//! use embive::{engine::{Engine, Config, SYSCALL_ARGS}, memory::{Memory, SliceMemory}, register::Register};
//!
//! /// A simple syscall example. Check [`engine::SyscallFn`] for more information.
//! fn syscall<M: Memory>(nr: i32, args: &[i32; SYSCALL_ARGS], memory: &mut M) -> Result<i32, i32> {
//!     println!("Syscall nr: {}, Args: {:?}", nr, args);
//!     match nr {
//!         1 => Ok(args[0] + args[1]), // Add two numbers (arg[0] + arg[1])
//!         2 => match memory.load(args[0] as u32) { // Load from RAM (arg[0])
//!             Ok(val) => Ok(i32::from_le_bytes(val)), // RISC-V is little endian
//!             Err(_) => Err(1),
//!         },
//!         _ => Err(2),
//!     }
//! }
//!
//! fn main() {
//!     // "10 + 20" using syscalls (load from ram and add two numbers)
//!     let code = &[
//!         0x93, 0x08, 0x20, 0x00, // li   a7, 2      (Syscall nr = 2)
//!         0x13, 0x05, 0x10, 0x00, // li   a0, 1      (a0 = 1)
//!         0x13, 0x15, 0xf5, 0x01, // slli a0, a0, 31 (a0 << 31) (0x80000000)
//!         0x73, 0x00, 0x00, 0x00, // ecall           (Syscall, load from arg0)
//!         0x93, 0x08, 0x10, 0x00, // li   a7, 1      (Syscall nr = 1)
//!         0x13, 0x05, 0x40, 0x01, // li   a0,20      (a0 = 20)
//!         0x73, 0x00, 0x00, 0x00, // ecall           (Syscall, add two args)
//!         0x73, 0x00, 0x10, 0x00  // ebreak          (Halt)
//!     ];
//!
//!     let mut ram = [0; 1024];
//!     // Store value 10 at RAM address 0 (0x80000000)
//!     ram[..4].copy_from_slice(&u32::to_le_bytes(10));
//!
//!     // Create memory from code and RAM slices
//!     let mut memory = SliceMemory::new(code, &mut ram);
//!
//!     // Create engine config
//!     let config = Config {
//!         syscall_fn: Some(syscall),
//!         ..Default::default()
//!     };
//!
//!     // Create engine & run it
//!     let mut engine = Engine::new(&mut memory, config).unwrap();
//!     engine.run().unwrap();
//!
//!     // Check the result (Ok(30))
//!     assert_eq!(engine.registers.get(Register::A0 as usize).unwrap(), 0);
//!     assert_eq!(engine.registers.get(Register::A1 as usize).unwrap(), 30);
//! }
//! ```
//!
//! ## Templates
//! The following templates are available for programs that run inside Embive:
//! - [Rust template](https://github.com/embive/embive-rust-template)
//! - [C/C++ Template](https://github.com/embive/embive-c-template)
//!
//! ## System Calls
//! System calls are a way for the untrusted code to interact with the host environment.
//! When provided to the engine, the system call function will be called when the `ecall` instruction is executed.
//! You can check more information about system calls in the [`engine::SyscallFn`] documentation.
//!
//! ## Features
//! Without any feature enabled, this crates has no external dependencies and can be used in a `no_std` & `no_alloc` environment.
//! Check the available features and their descriptions below:
//!
//! - `m_extension`:
//!     - Enable the RV32M extension (multiply and divide instructions).
//!         - Enabled by default, no additional dependencies.
//! - `instruction_limit`:
//!     - Limit the number of instructions executed by the engine, yielding when the limit is reached.
//!         - Disabled by default, no additional dependencies.
#![cfg_attr(not(test), no_std)]
pub mod engine;
pub mod error;
mod instruction;
pub mod memory;
pub mod register;

#[cfg(test)]
mod tests {
    use std::{fs::read_dir, path::PathBuf};

    use crate::{
        engine::{Config, Engine, SYSCALL_ARGS},
        memory::{SliceMemory, RAM_OFFSET},
    };

    const RAM_SIZE: usize = 16 * 1024;
    const TESTS_TO_RUN: usize = 48; // Amount of binaries to test

    thread_local! {
        static SYSCALL_COUNTER: std::cell::RefCell<i32> = std::cell::RefCell::new(0);
    }

    fn syscall(nr: i32, args: &[i32; SYSCALL_ARGS], _memory: &mut SliceMemory) -> Result<i32, i32> {
        if nr == 93 {
            if args[0] == 0 {
                println!("Test was successful");
            } else {
                panic!("Test failed at: {}", args[0]);
            }
        } else {
            panic!("Unknown syscall: {}", nr);
        }

        SYSCALL_COUNTER.with(|c| *c.borrow_mut() += 1);
        Ok(0)
    }

    #[test]
    fn riscv_binary_tests() {
        let code = &[];

        // Get all tests
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.push("tests");
        let tests = read_dir(dir).expect("Failed to read tests directory");

        // Iterate over tests
        let mut tested_files = 0;
        for test in tests {
            let test = test.expect("Failed to get test");

            // Check if it's a binary
            match test.path().extension() {
                Some(ext) if ext == "bin" => {}
                _ => continue, // Ignore other files
            }

            println!("\nRunning: {}", test.file_name().to_string_lossy());

            // Load binary into RAM
            let mut ram = [0; RAM_SIZE];
            let test_bytes = std::fs::read(test.path()).expect("Failed to read test file");
            ram[..test_bytes.len()].copy_from_slice(&test_bytes);

            let mut memory = SliceMemory::new(code, &mut ram);

            // Create engine
            let mut engine = Engine::new(
                &mut memory,
                Config {
                    syscall_fn: Some(syscall),
                    ..Default::default()
                },
            )
            .unwrap();

            // Set program counter to RAM (code start)
            engine.program_counter = RAM_OFFSET;

            // Get syscall counter prior to running
            let prev_syscall_counter = SYSCALL_COUNTER.with(|c| *c.borrow());

            // Run it
            loop {
                // println!("PC: {}", engine.program_counter());
                // println!("Registers: {:?}", engine.registers());
                // println!("Instruction: {:08X}", u32::from_le_bytes(engine.memory().load::<4>(engine.program_counter()).unwrap()));

                if !engine.step().unwrap() {
                    break;
                }
            }

            tested_files += 1;

            // Get syscall counter after running
            let new_syscall_counter = SYSCALL_COUNTER.with(|c| *c.borrow());

            // Check if syscall was incremented
            if new_syscall_counter <= prev_syscall_counter {
                panic!("No syscall was made");
            }
        }

        assert_eq!(tested_files, TESTS_TO_RUN);
    }
}
