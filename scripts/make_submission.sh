#!/usr/bin/env bash

# Clears the previous submissions.
rm -rf hw2.zip hw3.zip hw5.zip

# Creates new submissions.
zip hw2.zip -j hazardflow-designs/src/cpu/fetch.rs hazardflow-designs/src/cpu/decode.rs hazardflow-designs/src/cpu/exe.rs hazardflow-designs/src/cpu/branch_predictor/bht.rs hazardflow-designs/src/cpu/branch_predictor/btb.rs hazardflow-designs/src/cpu/branch_predictor/mod.rs
zip hw3.zip -j hazardflow-designs/src/cpu/fetch.rs hazardflow-designs/src/cpu/decode.rs hazardflow-designs/src/cpu/exe.rs hazardflow-designs/src/cpu/branch_predictor/bht.rs hazardflow-designs/src/cpu/branch_predictor/btb.rs hazardflow-designs/src/cpu/branch_predictor/mod.rs hazardflow-designs/src/cpu/riscv_isa.rs
zip hw5.zip -j hazardflow-designs/src/gemmini/execute/systolic_array/mesh.rs hazardflow-designs/src/gemmini/execute/systolic_array/transposer.rs
