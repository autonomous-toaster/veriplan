/* Capacity-0 channel: send and receive must meet. */
chan c = [0] of { bit };
bool got = false;

active proctype Sender() { c ! 1 }
active proctype Receiver() { c ? got }

ltl p0 { [] ( got -> got ) }
