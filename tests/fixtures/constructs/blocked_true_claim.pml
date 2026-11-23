/* A process blocked forever on a rendezvous channel that never meets, with a claim that holds.

   pan reports 0: invalid end state checking is disabled when a never claim is present, which every ltl
   claim produces. An engine that reports the deadlock as a violation disagrees with pan on every model
   that can block, which for this tool is every model - the emitter writes ltl claims for all constraints. */
chan c = [0] of { bit };
byte x = 0;

active proctype P() { c ? x }

ltl p0 { [] ( x == 0 ) }
