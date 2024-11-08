// Language: Verilog 2001

`timescale 1ns / 1ps

module fir (
    input  wire             clk,
    input  wire             rst,
    input  wire [8-1:0]     data_in,
    output wire [32-1:0]    data_out
);

// TODO: Implement this module.

parameter [7:0] b0 = 4;
parameter [7:0] b1 = 2;
parameter [7:0] b2 = 3;

reg [7:0] x1, x2;

assign data_out = b0 * data_in + b1 * x1 + b2 * x2;

always @ (posedge clk) begin
    if (rst) begin
        x1 <= 0;
        x2 <= 0;
    end else begin
        x2 <= x1;
        x1 <= data_in;
    end
end

endmodule
