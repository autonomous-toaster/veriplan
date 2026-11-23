/* An eval receive on a rendezvous channel, with a sender waiting. The engine refuses this by name since
   v0.5.7, which is the right answer whatever pan says.

   pan's answer here is 1, measured under plain and -a and with and without -DNOREDUCE, and under a guard
   position as well as a statement: <> ( got ) is refuted because the eval receive does not consume on a
   capacity-0 channel, so got stays false. The producer reported pan -a = 0 for this shape and called the
   engine's earlier verdict a wrong one; that 0 did not reproduce here, over six variants. Recorded with
   the measured answer so the row flags a disagreement in either direction rather than settling it by
   whichever of us asserted last. */
chan c = [0] of { bit };
bool got = false;

active proctype Sender() { c ! 1 }
active proctype Receiver() { c ? [1]; got = 1 }

ltl p0 { <> ( got ) }
