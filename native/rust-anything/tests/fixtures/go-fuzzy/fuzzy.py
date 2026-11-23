"""Near-duplicate names: every pair whose edit distance is at most `limit`."""


def levenshtein(a: str, b: str) -> int:
    previous = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        current = [i]
        for j, cb in enumerate(b, 1):
            current.append(min(previous[j] + 1, current[j - 1] + 1, previous[j - 1] + (ca != cb)))
        previous = current
    return previous[-1]


def dedupe(names: list[str], limit: int) -> list[list[str]]:
    pairs = []
    for i in range(len(names)):
        for j in range(i + 1, len(names)):
            if levenshtein(names[i], names[j]) <= limit:
                pairs.append([names[i], names[j]])
    return pairs
