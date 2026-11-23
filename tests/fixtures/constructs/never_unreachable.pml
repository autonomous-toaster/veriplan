/* A never claim whose arm no reachable state satisfies: pan reports no violation.

   Paired with never.pml, which carries `never { skip }` over the same model and which pan refutes.
   Only the claim body differs, so an implementation that reads "a never claim is present" as "report
   a violation" answers 1 for this file and is caught, while one that builds an acceptance condition
   answers 0. */
byte x = 0;

active proctype P() { x = 1 }

never { (x == 7) }
