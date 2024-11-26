//! Execute stage.

use super::*;

/// Payload from execute stage to memory stage.
#[derive(Debug, Clone, Copy)]
pub struct ExeEP {
    /// Writeback information.
    ///
    /// It contains the writeback address and selector.
    pub wb_info: HOption<(U<{ clog2(REGS) }>, WbSel)>,

    /// ALU output.
    pub alu_out: u32,

    /// Memory information.
    pub mem_info: HOption<MemInfo>,

    /// CSR information.
    pub csr_info: HOption<CsrInfo>,

    /// Indicates that the instruction is illegal or not.
    pub is_illegal: bool,

    /// PC.
    pub pc: u32,

    /// Instruction (for debugging purpose).
    pub debug_inst: u32,
}

/// Hazard from execute stage to decode stage.
#[derive(Debug, Clone, Copy)]
pub struct ExeR {
    /// Bypassed data from EXE.
    pub bypass_from_exe: HOption<Register>,

    /// Bypassed data from MEM.
    pub bypass_from_mem: HOption<Register>,

    /// Bypassed data from WB.
    pub bypass_from_wb: HOption<Register>,

    /// Stall.
    ///
    /// It contains the rd address of load or CSR instructions.
    pub stall: HOption<U<{ clog2(REGS) }>>,

    /// Indicates that the pipeline should be redirected.
    pub redirect: HOption<u32>,

    /// Register file.
    pub rf: Regfile,

    /// Branch predictor update signal.
    pub bp_update: HOption<BpUpdate>
}

impl ExeR {
    /// Creates a new execute resolver.
    pub fn new(
        memr: MemR,
        bypass: HOption<Register>,
        stall: HOption<U<{ clog2(REGS) }>>,
        redirect: HOption<u32>,
        bp_update: HOption<BpUpdate>,
    ) -> Self {
        Self {
            bypass_from_exe: bypass,
            bypass_from_mem: memr.bypass_from_mem,
            bypass_from_wb: memr.bypass_from_wb,
            stall,
            redirect: memr.redirect.or(redirect),
            rf: memr.rf,
            bp_update: bp_update,
        }
    }
}

/// Returns redirected PC based on the given payload.
fn get_redirect(p: DecEP, alu_out: u32) -> (HOption<u32>, HOption<BpUpdate>) {
    let Some(br_info) = p.br_info else {
        return (None, None);
    };

    let target = br_info.base + br_info.offset;
    let alu_true = alu_out != 0;

    match br_info.typ {
        // Instruction is JAL
        BrType::Jal => (None, None),

        // Instruction is JALR
        BrType::Jalr => {
            // Prediction is true
            if target == p.bp_result.btb {
                (None, None)
            }
            // Mispredicted 
            else {
                let bp_update = BpUpdate::Btb { pc: p.pc, target };
                (Some(target), Some(bp_update))
            }
        },

        // Instruction is Branch if (greater than or) equal
        BrType::Beq | BrType::Bge | BrType::Bgeu => {
            // Branch resolved as taken
            if !alu_true {
                let bp_update = BpUpdate::Bht { pc: p.pc, taken: true };
                // Predicted as taken
                if p.bp_result.bht {
                    (None, Some(bp_update))
                }
                // Predicted as not taken -> mispredicted -> redirect to target
                else {
                    (Some(target), Some(bp_update))
                }
            } 

            // Branch resolve as not taken
            else {                        
                let bp_update = BpUpdate::Bht { pc: p.pc, taken: false };
                // Predicted as taken -> mispredicted -> redirected to current PC + 4
                if p.bp_result.bht {
                    (Some(p.pc + 4), Some(bp_update))
                }
                // Predicted as not taken 
                else {
                    (None, Some(bp_update))
                }
            }
        }

        // Instruction is Branch if less than
        BrType::Bne | BrType::Blt | BrType::Bltu => {
            // Branch resolved as taken
            if alu_true {
                let bp_update = BpUpdate::Bht { pc: p.pc, taken: true };
                // Predicted as taken
                if p.bp_result.bht {
                    (None, Some(bp_update))
                }
                // Predicted as not taken -> mispredicted -> redirect to target
                else { 
                    (Some(target), Some(bp_update))
                }

            // Branch resolved as not taken
            } else {                        
                let bp_update = BpUpdate::Bht { pc: p.pc, taken: false };
                // Predicted as taken -> mispredicted -> redirect to current PC + 4
                if p.bp_result.bht {
                    (Some(p.pc + 4), Some(bp_update))
                } 
                // Predicted as not taken
                else {
                    (None, Some(bp_update))
                }
            }
        }
    }
}

/// Generates resolver from execute stage to decode stage.
fn gen_resolver(er: (HOption<(DecEP, u32)>, MemR)) -> ExeR {
    let (p, memr) = er;

    let stall = p.and_then(|(p, _)| {
        p.wb_info.and_then(|(addr, wb_sel)| if matches!(wb_sel, WbSel::Mem | WbSel::Csr) { Some(addr) } else { None })
    });

    let Some((p, alu_out)) = p else {
        return ExeR::new(memr, None, stall, None, None);
    };

    let bypass =
        p.wb_info.and_then(
            |(addr, wb_sel)| if matches!(wb_sel, WbSel::Alu) { Some(Register::new(addr, alu_out)) } else { None },
        );

    let (redirect, bp_update) = get_redirect(p, alu_out);

    ExeR::new(memr, bypass, stall, redirect, bp_update)
}

/// Generates payload from execute stage to memory stage.
fn gen_payload(ip: DecEP, alu_out: u32, memr: MemR) -> HOption<ExeEP> {
    if memr.redirect.is_some() {
        None
    } else {
        Some(ExeEP {
            alu_out,
            wb_info: ip.wb_info,
            mem_info: ip.mem_info,
            csr_info: ip.csr_info,
            is_illegal: ip.is_illegal,
            pc: ip.pc,
            debug_inst: ip.debug_inst,
        })
    }
}

/// inner Execute stage.
fn inner_exe(
    i : I<VrH<DecEP, (HOption<(DecEP, u32)>, MemR)>, {Dep::Demanding}>,
) ->  I<VrH<(DecEP, u32), MemR>, { Dep::Demanding }> {
    let deep = i
        .reg_fwd(true)
        .map_resolver_inner(|er: ((HOption<(DecEP, u32)>, MemR), (HOption<(DecEP, u32)>, MemR))| {
            let (alu_r, mext_r) = er;
            if alu_r.0.is_some() {
                alu_r
            } else {
                mext_r
            }
        });

    let (alu_req, mext_req) = deep
        .map(|p| {
            let op = p.alu_input.op;
            let sel = match op {
                AluOp::Base(_) => 0.into_u(),
                AluOp::Mext(_) => 1.into_u(),
            };
            
            (p, BoundedU::new(sel))
        })
        .branch();

    let alu_resp = alu_req
        .map(|p| match p.alu_input.op {
            AluOp::Base(op) => (p, exe_alu(p.alu_input.op1_data, p.alu_input.op2_data, op)),
            AluOp::Mext(_) => todo!("assignment 3"),
        })
        .map_resolver_block_with_p::<VrH<(DecEP, u32), MemR>>(|ip, er| (ip, er.inner));

    let mext_resp = mext_req
        .map(|p| match p.alu_input.op {
            AluOp::Base(_) => todo!("never happen"),
            AluOp::Mext(op) => {
                let mul_req = MulReq {
                    op,
                    in1: From::from(p.alu_input.op1_data),
                    in2: From::from(p.alu_input.op2_data),
                };
                (p, mul_req)
            },
        })
        .comb(muldiv)
        .map(|p| (p.0, u32::from(p.1)))
        .map_resolver_inner::<(HOption<(DecEP, u32)>, MemR)>(|er| {
            let redirect = er.1.redirect;
            match redirect {
                Some(_) => (er, true),
                None => (er, false),
            }
        })
        .map_resolver_block_with_p::<VrH<(DecEP, u32), MemR>>(|ip, er| (ip, er.inner));

    [alu_resp, mext_resp].merge()

}


/// Execute stage.
pub fn exe(i: I<VrH<DecEP, ExeR>, { Dep::Demanding }>) -> I<VrH<ExeEP, MemR>, { Dep::Demanding }> {
    i.map_resolver_inner::<(HOption<(DecEP, u32)>, MemR)>(gen_resolver)
        .comb(exclusive(inner_exe))
        .filter_map_drop_with_r_inner(|(ip, alu_out), er| gen_payload(ip, alu_out, er))
}
