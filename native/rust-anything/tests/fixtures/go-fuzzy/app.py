import sys

from fuzzy import dedupe

FIRST = ["anna", "ben", "carla", "dmitri", "elif", "farid", "greta", "hugo", "ines", "jonas"]
LAST = ["yilmaz", "smith", "kovacs", "novak", "silva", "berg", "rossi", "lind", "moreau", "kaya"]


def names(n: int) -> list[str]:
    out = []
    for k in range(n):
        name = f"{FIRST[k % 10]} {LAST[(k * 7) % 10]}{k // 100}"
        if k % 3 == 0:
            name = name[:-2] + name[-1] + name[-2]
        out.append(name)
    return out


if __name__ == "__main__":
    pairs = dedupe(names(int(sys.argv[1]) if len(sys.argv) > 1 else 400), 1)
    print(len(pairs), "near-duplicate pairs")
