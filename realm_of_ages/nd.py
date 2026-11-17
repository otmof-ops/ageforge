"""The few image operations the world needs, from SciPy when it is there and in plain NumPy and Python when it is
not (as under PyPy, which SciPy does not build for). Both give the same results.

gaussian_filter, binary_dilation, distance_transform_edt (exact, with nearest-feature indices), label (4-connected,
like SciPy's default), sum and center_of_mass over labels."""
import math

import numpy as np

try:                                                                     # CPython: SciPy's C code
    from scipy import ndimage as _nd
except ImportError:                                                      # PyPy: the fallbacks below
    _nd = None


def gaussian_filter(a, sigma):
    if _nd is not None:
        return _nd.gaussian_filter(a, sigma)
    r = int(4 * sigma + 0.5)
    x = np.arange(-r, r + 1, dtype=float)
    k = np.exp(-0.5 * (x / sigma) ** 2)
    k /= k.sum()
    out = np.asarray(a, dtype=float)
    for axis in (0, 1):
        pad = [(0, 0), (0, 0)]
        pad[axis] = (r, r)
        p = np.pad(out, pad, mode="symmetric")                          # SciPy's default 'reflect'
        acc = np.zeros_like(out)
        n = out.shape[axis]
        for i, w in enumerate(k):
            acc += w * (p[i:i + n] if axis == 0 else p[:, i:i + n])
        out = acc
    return out


def binary_dilation(a):
    if _nd is not None:
        return _nd.binary_dilation(a)
    a = np.asarray(a, dtype=bool)
    out = a.copy()
    out[1:] |= a[:-1]
    out[:-1] |= a[1:]
    out[:, 1:] |= a[:, :-1]
    out[:, :-1] |= a[:, 1:]
    return out


def _edt_1d(f, n):
    """Felzenszwalb-Huttenlocher: squared distance to the nearest feature along one line, and which one."""
    INF = 1e20
    v = [0] * n
    z = [0.0] * (n + 1)
    d = [0.0] * n
    arg = [0] * n
    k = 0
    v[0] = 0
    z[0], z[1] = -INF, INF
    for q in range(1, n):
        if f[q] >= INF:
            continue
        if f[v[0]] >= INF:
            v[0] = q
            continue
        while True:
            p = v[k]
            s = ((f[q] + q * q) - (f[p] + p * p)) / (2 * q - 2 * p)
            if s <= z[k]:
                k -= 1
                if k < 0:
                    k = 0
                    v[0] = q
                    z[0], z[1] = -INF, INF
                    break
                continue
            k += 1
            v[k] = q
            z[k] = s
            z[k + 1] = INF
            break
    if f[v[0]] >= INF:
        return [INF] * n, [-1] * n
    k = 0
    for q in range(n):
        while z[k + 1] < q:
            k += 1
        p = v[k]
        d[q] = (q - p) * (q - p) + f[p]
        arg[q] = p
    return d, arg


def distance_transform_edt(a, return_indices=False):
    """Distance from every non-zero cell to the nearest zero cell (and, if asked, that cell's coordinates)."""
    if _nd is not None:
        return _nd.distance_transform_edt(a, return_indices=return_indices)
    a = np.asarray(a)
    H, W = a.shape
    INF = 1e20
    feat = (a == 0)
    col_d = np.empty((H, W))
    col_i = np.empty((H, W), dtype=np.int64)
    for x in range(W):
        f = [0.0 if feat[y, x] else INF for y in range(H)]
        d, arg = _edt_1d(f, H)
        col_d[:, x] = d
        col_i[:, x] = arg
    dist = np.empty((H, W))
    iy = np.empty((H, W), dtype=np.int64)
    ix = np.empty((H, W), dtype=np.int64)
    for y in range(H):
        f = col_d[y].tolist()
        d, arg = _edt_1d(f, W)
        dist[y] = d
        ix[y] = arg
        iy[y] = col_i[y, arg]
    dist = np.sqrt(dist)
    if return_indices:
        return dist, np.stack([iy, ix])
    return dist


def label(a):
    """4-connected components of the non-zero cells, numbered from 1 in scan order, as SciPy does."""
    if _nd is not None:
        return _nd.label(a)
    a = np.asarray(a, dtype=bool)
    H, W = a.shape
    lab = np.zeros((H, W), dtype=np.int32)
    cells = a.tolist()
    out = [[0] * W for _ in range(H)]
    n = 0
    for y in range(H):
        row = cells[y]
        for x in range(W):
            if row[x] and not out[y][x]:
                n += 1
                out[y][x] = n
                stack = [(y, x)]
                while stack:
                    cy, cx = stack.pop()
                    for ny, nx in ((cy - 1, cx), (cy + 1, cx), (cy, cx - 1), (cy, cx + 1)):
                        if 0 <= ny < H and 0 <= nx < W and cells[ny][nx] and not out[ny][nx]:
                            out[ny][nx] = n
                            stack.append((ny, nx))
    lab[:] = out
    return lab, n


def sum(a, labels, index):
    if _nd is not None:
        return _nd.sum(a, labels, index=index)
    s = np.bincount(labels.ravel(), weights=np.asarray(a, dtype=float).ravel(), minlength=int(np.max(index)) + 1)
    return s[np.asarray(index)]


def center_of_mass(a, labels, index):
    if _nd is not None:
        return _nd.center_of_mass(a, labels, index=index)
    w = np.asarray(a, dtype=float).ravel()
    lab = labels.ravel()
    H, W = labels.shape
    yy, xx = np.indices((H, W))
    m = int(np.max(index)) + 1
    tot = np.bincount(lab, weights=w, minlength=m)
    sy = np.bincount(lab, weights=w * yy.ravel(), minlength=m)
    sx = np.bincount(lab, weights=w * xx.ravel(), minlength=m)
    return [(sy[i] / tot[i], sx[i] / tot[i]) if tot[i] else (math.nan, math.nan) for i in index]
