/* Ordering through a rendezvous: the flag is only set after the handshake, so pan can decide
   a claim about what must be true before it. */
chan c = [0] of { bit };
bool handshake = false;
bool after = false;

active proctype Sender() { c ! 1; after = true }
active proctype Receiver() { c ? handshake }

ltl p0 { [] ( after -> handshake ) }
