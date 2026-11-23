/* The control: no sender, so the poll cannot succeed and got stays 0. pan reports 1 under -a, because a
   liveness claim is only decided when acceptance cycles are looked for. Plain pan reports 0 for both
   files, so -a is the flag that makes this pair mean anything. */
chan c = [0] of { bit };
byte got = 0;

active proctype Receiver() { c ?? 1; got = 1 }

ltl p0 { <> ( got == 1 ) }
