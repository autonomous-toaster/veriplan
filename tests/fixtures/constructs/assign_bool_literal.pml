bool flag = false;
active proctype P() { flag = true }
ltl p0 { [] ( !flag ) }
