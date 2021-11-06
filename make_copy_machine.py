def main():
    syms = ['_'] + [str(s) for s in input().split(' ')]
    states = ['init'] + [f"q{s}" for s in syms]
    print("# copies the current symbol and moves it to the left")
    print(f"states {' '.join(states)}")
    print(f"syms {' '.join(syms)}")
    print("initstate init")
    print("finalstates HALT")
    print("table")
    for sym in syms:
        print(f"init {sym} q{sym} . L")
        print(f"q{sym} * HALT {sym} R")

if __name__ == '__main__':
    main()