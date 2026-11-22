/* A capacity-0 channel with a claim pan decides: the send and the receive must meet, so got
   becomes true and the liveness claim holds. */
chan c = [0] of { bit };
bool got = false;

active proctype Sender() { c ! 1 }
active proctype Receiver() { c ? got }

ltl p0 { <> ( got ) }
