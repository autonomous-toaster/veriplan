/* A silent pass, which is worse than a wrong verdict: the engine reports no violation having explored zero
   states, where pan refutes the assertion. An initialised mtype declaration inside a body is what does it;
   the same assignment as a separate statement decides correctly (mtype_numbering.pml).

   pan 1, engine 0 with states_explored 0. Since nothing is reported as unverified, a caller counting
   verdicts reads this as satisfied. */
mtype = { a, b, c };

active proctype P() { mtype x = a; assert(x == 1) }
