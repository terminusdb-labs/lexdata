import math
from fractions import Fraction

def num_convert(lst):
    z = 0
    for i in range(0,len(lst)):
        if i != 0:
            z = z * 2 ** 8
        z = z + lst[i]
    return z

num = num_convert([9, 0, 0, 0, 0, 0, 0, 0, 9])
print(f"{num}")


def continued(n,d):
    N_nm1 = n/d
    N_n = 1
    N_np1 = N_nm1 % N_n
    seq = [math.floor(N_nm1)]
    while N_np1 > 0:
        approx = math.floor( N_n / N_np1 )
        seq.append(approx)
        N_nm1 = N_n
        N_n = N_np1
        N_np1 = N_nm1 % N_n
    return seq

def normalize(seq):
    if len(seq) > 2:
        last = seq[-1]
        if last == 1:
            seq = seq[:-2] + [seq[-2]+1]
    return seq

def convergent(seq):
    a_z = seq[0]
    newseq = seq[0:].copy()
    newseq.reverse()
    number = 0
    for i in range(0,len(newseq)):
        a_n = newseq.pop(0)
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


vals = [(1,3),(3,5),(23,12),(22,7),(5,2),(10,1),(8,7),(-2,3),(-400,1)]

for (n,d) in vals:
    (p,q) = stern_brocot_to_nd(nd_to_stern_brocot(n,d))
    #assert (p == n and d == q)

seqs = []
for i in range(0,len(vals)):
    (n,d) = vals.pop(0)
    print(f"n/d: {n}/{d}")
    c = nd_to_stern_brocot(n,d)
    print(f"seq: {c}")
    seqs.append(c)

seqs.sort()

convs = []
for i in range(0,len(seqs)):
    seq = seqs.pop(0)
    (n,d) = stern_brocot_to_nd(seq)
    convs.append(n/d)

print(f"convs: {convs}")

def date_to_stern_brocot(date):
    f = Fraction.from_float(date)
    p = f.numerator
    q = f.denominator
    print(f"p/q: {p}/{q}")
    return nd_to_stern_brocot(p,q)

def stern_brocot_to_date(stern_brocot_seq):
    (n,d) = stern_brocot_to_nd(stern_brocot_seq)
    return n / d


# import time

# now = time.time()
# print(f"date-time: {now}")
# sb_now = date_to_stern_brocot(now)
# print(f"Stern-Brocot date-time: {sb_now}")
# now_again = stern_brocot_to_date(sb_now)
# print(f"date-time again: {now_again}")

