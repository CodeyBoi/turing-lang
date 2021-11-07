def main():
    syms = ['_'] + [str(s) for s in input().split(' ')]
    states = ['init'] + [f"q{i}" for i in range(len(syms))]
    print("# copies the current symbol and moves it to the left")
    print(f"states {' '.join(states)}")
    print(f"syms {' '.join(syms)}")
    print("initstate init")
    print("finalstates HALT")
    print("table")
    for i, sym in enumerate(syms):
        print(f"init {sym} q{i} . L")
        print(f"q{i} * HALT {sym} R")

if __name__ == '__main__':
    main()