/* A poll on a rendezvous channel with a sender waiting: got becomes 1, so the liveness claim holds.
   pan 0 under both plain and -a. Paired with poll_no_sender.pml, where nobody is sending. */
chan c = [0] of { bit };
byte got = 0;

active proctype Sender() { c ! 1 }
active proctype Receiver() { c ?? 1; got = 1 }

ltl p0 { <> ( got == 1 ) }
