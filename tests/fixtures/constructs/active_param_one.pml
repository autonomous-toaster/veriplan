/* Control for active_param_zero.pml: the same model asserting n == 1, which any implementation that binds a
   non-zero default, or none, refutes. pan 1. */
active proctype P(byte n) { assert(n == 1) }
