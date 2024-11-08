//! Fetch stage.

use super::*;

/// Payload from fetch stage to decode stage.
#[derive(Debug, Clone, Copy)]
pub struct FetEP {
    /// IMEM response.
    pub imem_resp: MemRespWithAddr,
    /// Branch prediction result.
    pub bp_result: BpResult,
    /// Branch predictor update.
    pub bp_update: HOption<BpUpdate>,
}



/// Fetch stage.
pub fn fetch<const START_ADDR: u32>(
    imem: impl FnOnce(Vr<MemReq>) -> Vr<MemRespWithAddr>,
) -> I<VrH<FetEP, DecR>, { Dep::Demanding }> {
    // next PC calculation
    let next_pc = <I<VrH<(HOption<FetEP>, DecR), _>, { Dep::Demanding }>>::source_drop()
        .filter_map(|(p, decr)| {
            let DecR { redirect, bp_update } = decr;
            
            // Next PC calculation based on the branch prediction
            match redirect {
                // Next PC is redirected by later stage
                Some(target) => Some((target, bp_update)),

                // Else
                None => {
                    match p {
                        Some(fet_ep) => {
                            let current_pc = fet_ep.imem_resp.addr;
                            let bp_result = fet_ep.bp_result;
                            let pre_decode = bp_result.pre_decode;
                            let imm = u32::from(pre_decode.imm);

                            // Current instruction is predecoded as JAL -> next PC = current PC + imm
                            if pre_decode.is_jal {
                                Some((current_pc + imm, bp_update))
                            }

                            // Current instruction is predecoded as JALR
                            else if pre_decode.is_jalr {
                                // BTB predicted next PC as current PC + 4 -> BTB miss -> next PC = current PC + 4 = BTB
                                // BTB predicted next PC as target -> BTB hit -> next PC = target = BTB
                                Some((bp_result.btb, bp_update))
                            }

                            // Current instruction is predecoded as branching
                            else if pre_decode.is_branch {
                                // BHT = taken -> next PC = current PC + imm
                                if bp_result.bht {
                                    Some((current_pc + imm, bp_update))
                                // BHT = not taken -> next PC = current PC + 4
                                } else {
                                    Some((current_pc + 4, bp_update))
                                }
                            }
                            
                            // Other -> next PC = current PC + 4
                            else {
                                Some((current_pc + 4, bp_update))
                            }
                        },
                        None => None,
                    }
                },
            }
        })
        .reg_fwd_with_init(true, (START_ADDR, None));
    

    // Default BpResult
    let pre_decode = pre_decode(Array::<bool, 32>::from([false; 32]));
    let default_bp_res = BpResult {
        pre_decode,
        bht: false,
        btb: 0,
    };

    // Attach branch update to IMEM payload
    let imem_with_update = attach_payload::<MemReq, MemRespWithAddr, HOption<BpUpdate>>(imem);

    // Fetch
    next_pc
        .map(|(pc, bp_update)| (MemReq::load(pc, MemOpTyp::WU), bp_update))

        .comb::<I<VrH<(MemRespWithAddr, HOption<BpUpdate>), _>, { Dep::Helpful }>>(attach_resolver(imem_with_update))

        // bp_result is generated at M4, this bp_update is resolved at EXE stage: ExeR -> DecR -> FetEP.
        .map(|(imem_resp, bp_update)| FetEP { imem_resp, bp_result: default_bp_res, bp_update })    

        .fsm_map(Bp::default(), |ip, s| {
            // Make a branch prediction based on the IMEM response
            let bp_result = s.predict(ip.imem_resp);     
            
            // Attach it to the egress payload
            let ep = FetEP {
                imem_resp: ip.imem_resp,
                bp_result,
                bp_update: None,       // bp_update is generated at EXE stage.
            };
            
            // Update branch predictor based on the branch resolve result
            let s1 = match ip.bp_update {
                Some(update) => s.update(update),
                None => s,
            };

            (ep, s1)
        })

        .map_resolver_drop_with_p::<VrH<FetEP, DecR>>(|ip, er| {
            let DecR { redirect, .. } = er.inner;
            // We need `kill` here to extract the mispredicted PC from register, and then filter out them.
            Ready::new(er.ready || redirect.is_some(), (ip, er.inner))
        })

        .filter_map_drop_with_r_inner(|resp, er| if er.redirect.is_none() { Some(resp) } else { None })
}