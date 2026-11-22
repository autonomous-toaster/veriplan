/* The same model with the opposite claim: pan refutes, because got does become true. */
chan c = [0] of { bit };
bool got = false;

active proctype Sender() { c ! 1 }
active proctype Receiver() { c ? got }

ltl p0 { [] ( !got ) }
