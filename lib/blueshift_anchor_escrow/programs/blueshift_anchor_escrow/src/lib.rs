use anchor_lang::prelude::*;

mod errors;
mod instructions;
mod state;

use instructions::*;

declare_id!("22222222222222222222222222222222222222222222");

#[program]
pub mod anchor_escrow {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn make(ctx: Context<Make>, seed: u64, deposit: u64, receive: u64) -> Result<()> {
        instructions::make::handler(ctx, seed, deposit, receive)
    }

    #[instruction(discriminator = 1)]
    pub fn take(ctx: Context<Take>) -> Result<()> {
        instructions::take::handler(ctx)
    }

    #[instruction(discriminator = 2)]
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        instructions::refund::handler(ctx)
    }
}