import math
import time
from fractions import Fraction

def num_convert(lst):
    z = 0
    for i in range(0,len(lst)):
        if i != 0:
            z = z * 2 ** 8
        z = z + lst[i]
    return z

#num = num_convert([9, 0, 0, 0, 0, 0, 0, 0, 9])
#print(f"{num}")

def continued(n,d):
    N_nm1 = Fraction(n,d)
    N_n = Fraction(1,1)
    N_np1 = N_nm1 % N_n
    seq = [math.floor(N_nm1)]
    while N_np1 > 0:
        approx = math.floor( N_n / N_np1 )
        seq.append(approx)
        N_nm1 = N_n
        N_n = N_np1
        N_np1 = N_nm1 % N_n
    return seq

def canonical_even(seq):
    l = len(seq)
    if (l % 2) == 0 or seq == [0]:
        return seq
    else:
        
        return seq

def nd_lexical(n,d):
    seq = continued(n,d)
    seq = canonical_even(seq)
    return alternate(seq)

def alternate(seq):
    pos = True
    new = []
    for n in seq:
        if pos == True:
            pos = False
            new.append(n)
        else:
            pos = True
            new.append(-n)
    return new

def lexical_nd(seq):
    newseq = alternate(seq)
    a_z = newseq.pop(0)
    newseq.reverse()
    number = 0
    for a_n in newseq:
        print(f"a_n: {a_n}")
        if number == 0:
            number = Fraction(1,a_n)
        else:
            number = Fraction(1, (a_n + number))
    f = a_z + number
    n = f.numerator
    d = f.denominator
    return (n,d)

def normalize(seq):
    if len(seq) > 2:
        last = seq[-1]
        if last == 1:
            seq = seq[:-2] + [seq[-2]+1]
    return seq

def convergent(seq):
    a_z = seq[0]
    newseq = seq[1:].copy()
    newseq.reverse()
    number = 0
    for a_n in newseq:
        print(f"a_n: {a_n}")
        if number == 0:
            number = 1/a_n
        else:
            number = 1 / (a_n + number)
    return a_z + number

def normalize_fraction(n,d):
    fraction = Fraction(n,d)
    n = fraction.numerator
    d = fraction.denominator
    s = 1
    if n < 0:
        n = -n
        s = 0
    return(s,n,d)

def nd_to_stern_brocot(n,d):
    (s,n,d) = normalize_fraction(n,d)
    p = 1
    q = 1
    path = [1] # sign bit
    p_left = 0
    q_left = 1
    p_right = 1
    q_right = 0
    while n / d != p / q:
        # print(f"{p}/{q} ",end='')
        if p / q > n / d:
            p_right = p
            q_right = q
            p = p_left + p
            q = q_left + q
            path.append(0)
        elif p / q < n / d:
            p_left = p
            q_left = q
            p = p_right + p
            q = q_right + q
            path.append(1)
    if s == 0:
        path = flip_bits(path)
    return path

def flip_bits(path):
    new_path = []
    for elt in path:
        if elt == 0:
            new_path.append(1)
        elif elt == 1:
            new_path.append(0)
        else:
            raise Exception("Not a valid Stern-Brocot Path")
    return new_path

def stern_brocot_to_nd(brocot_seq):
    sign = brocot_seq.pop(0)
    if sign == 0:
        brocot_seq = flip_bits(brocot_seq)
    p = 1
    q = 1
    p_left = 0
    q_left = 1
    p_right = 1
    q_right = 0
    for bit in brocot_seq:
        if bit == 0:
            p_right = p
            q_right = q
            p = p_left + p
            q = q_left + q
        else:
            p_left = p
            q_left = q
            p = p_right + p
            q = q_right + q
    if sign == 0:
        p = -p
    return (p,q)

## Test vals
vals = [(1,3),(3,5),(23,12),(22,7),(5,2),(10,1),(8,7),(3432,233)]

## Continued fractions
for (n,d) in vals:
    (p,q) = lexical_nd(nd_lexical(n,d))
    assert (p == n and d == q)

continued_vals = []
for (n,d) in vals:
    print(f"n/d: {n}/{d}")
    seq = nd_lexical(n,d)
    print(f"seq: {seq}")
    continued_vals.append(seq)

print(continued_vals)
continued_vals.sort()
sorted_vals = []
for seq in continued_vals:
    (n,d) = lexical_nd(seq)
    sorted_vals.append(Fraction(n,d))

assert(sorted(sorted_vals) == sorted_vals)
print(f"lexicalically sorted: {sorted_vals}")

def date_to_continued(date):
    f = Fraction.from_float(date)
    p = f.numerator
    q = f.denominator
    print(f"p/q: {p}/{q}")
    return continued(p,q)

def continued_to_date(continued_seq):
    return convergent(continued_seq)

# import time
now = time.time()
print(f"date-time: {now}")
sb_now = date_to_continued(now)
print(f"continued date-time: {sb_now}")
now_again = continued_to_date(sb_now)
print(f"date-time again: {now_again}")


# Stern Brocot examples
for (n,d) in vals:
    (p,q) = stern_brocot_to_nd(nd_to_stern_brocot(n,d))
    assert (p == n and d == q)

stern_brocot_seqs = []
stern_vals = vals.copy()
for (n,d) in stern_vals:
    print(f"n/d: {n}/{d}")
    c = nd_to_stern_brocot(n,d)
    print(f"seq: {c}")
    stern_brocot_seqs.append(c)

stern_brocot_seqs.sort()

convs = []
for seq in stern_brocot_seqs:
    (n,d) = stern_brocot_to_nd(seq)
    convs.append(n/d)
print(f"convs: {convs}")


