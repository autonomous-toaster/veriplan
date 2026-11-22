byte x = 0;

proctype Worker() { x = 1 }

active proctype Main() {
    run Worker();
    skip
}

ltl p0 { [] ( x <= 1 ) }
