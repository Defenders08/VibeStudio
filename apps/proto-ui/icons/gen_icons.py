#!/usr/bin/env python3
"""Генератор собственных монохромных иконок VibeStudio (замена Image-Line).
Белый глиф на прозрачном фоне, RGBA PNG. Примитивы: линии-диски, круги, полигоны."""

import zlib, struct, math, os

def png_write(path, w, h, px):  # px[y][x] = (r,g,b,a)
    raw = b"".join(b"\x00" + b"".join(bytes(px[y][x]) for x in range(w)) for y in range(h))
    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    hdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", hdr)
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    open(path, "wb").write(data)

class C:
    def __init__(self, size):
        self.s = size
        self.px = [[(255, 255, 255, 0)] * size for _ in range(size)]
        self.k = size / 80.0  # координаты задаются в сетке 80×80
    def put(self, x, y, a=255):
        x, y = int(round(x * self.k)), int(round(y * self.k))
        if 0 <= x < self.s and 0 <= y < self.s:
            self.px[y][x] = (255, 255, 255, a)
    def disc(self, cx, cy, r):
        R, cxk, cyk = r * self.k, cx * self.k, cy * self.k
        for y in range(self.s):
            for x in range(self.s):
                if (x - cxk) ** 2 + (y - cyk) ** 2 <= R * R:
                    self.px[y][x] = (255, 255, 255, 255)
    def line(self, x0, y0, x1, y1, w):
        n = int(max(abs(x1 - x0), abs(y1 - y0)) * self.k * 2) + 2
        for i in range(n + 1):
            t = i / n
            self.disc(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, w / 2)
    def poly(self, pts):
        # заполнение по scanline
        xs = [p[0] * self.k for p in pts]; ys = [p[1] * self.k for p in pts]
        for y in range(self.s):
            inter = []
            for i in range(len(pts)):
                a, b = (xs[i], ys[i]), (xs[(i + 1) % len(pts)], ys[(i + 1) % len(pts)])
                if (a[1] <= y < b[1]) or (b[1] <= y < a[1]):
                    inter.append(a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]))
            inter.sort()
            for i in range(0, len(inter) - 1, 2):
                for x in range(int(inter[i]), int(inter[i + 1]) + 1):
                    if 0 <= x < self.s:
                        self.px[y][x] = (255, 255, 255, 255)
    def ring(self, cx, cy, r, w, a0=0, a1=360):
        for i in range(int(a0 * 4), int(a1 * 4)):
            ang = math.radians(i / 4)
            self.disc(cx + r * math.cos(ang), cy + r * math.sin(ang), w / 2)
    def rect_o(self, x0, y0, x1, y1, w):
        self.line(x0, y0, x1, y0, w); self.line(x1, y0, x1, y1, w)
        self.line(x1, y1, x0, y1, w); self.line(x0, y1, x0, y0, w)
    def dashed_rect(self, x0, y0, x1, y1, w):
        for (ax, ay, bx, by) in [(x0, y0, x1, y0), (x1, y0, x1, y1), (x1, y1, x0, y1), (x0, y1, x0, y0)]:
            L = math.hypot(bx - ax, by - ay); on = True
            i = 0
            while i < L:
                seg = 6
                if on:
                    t1 = min(i + seg, L)
                    for j in range(int(i * 2), int(t1 * 2)):
                        t = j / 2 / L
                        self.disc(ax + (bx - ax) * t, ay + (by - ay) * t, w / 2)
                i += seg + 4; on = not on

OUT = os.path.join(os.path.dirname(__file__), "own")
os.makedirs(OUT, exist_ok=True)

def save(name, size, fn):
    c = C(size); fn(c); png_write(os.path.join(OUT, name + ".png"), size, size, c.px)

# --- инструменты (80×80) ---
save("draw", 80, lambda c: (c.line(22, 58, 56, 24, 7), c.poly([(18,62),(30,50),(26,66)]), c.line(50,20,62,32,7)))
save("paint", 80, lambda c: (c.disc(46, 52, 11), c.line(50, 46, 64, 20, 6), c.disc(64, 18, 4)))
save("paint_drum", 80, lambda c: (c.line(22, 22, 46, 46, 6), c.line(58, 22, 34, 46, 6), c.line(40, 56, 52, 68, 6), c.line(56, 56, 44, 68, 6)))
save("delete", 80, lambda c: (c.rect_o(28, 32, 52, 66, 5), c.line(24, 32, 56, 32, 5), c.line(36, 26, 44, 26, 5), c.line(36, 26, 36, 32, 5), c.line(44, 26, 44, 32, 5), c.line(38, 40, 38, 58, 5), c.line(46, 40, 46, 58, 5)))
save("mute", 80, lambda c: (c.poly([(20,36),(30,36),(40,26),(40,54),(30,44),(20,44)]), c.line(48, 38, 62, 52, 6), c.line(62, 38, 48, 52, 6)))
save("slice", 80, lambda c: (c.poly([(20,62),(56,62),(56,34)]), c.line(56, 32, 66, 18, 6)))
save("select", 80, lambda c: c.dashed_rect(18, 18, 62, 62, 5))
save("playback", 80, lambda c: (c.poly([(18,36),(28,36),(38,26),(38,54),(28,44),(18,44)]), c.ring(46, 40, 10, 4, -60, 60), c.ring(46, 40, 18, 4, -50, 50)))
save("play", 80, lambda c: c.poly([(30, 22), (30, 58), (60, 40)]))
save("note", 80, lambda c: (c.disc(34, 58, 8), c.line(42, 58, 42, 20, 5), c.line(42, 20, 58, 28, 5)))
save("portamento", 80, lambda c: (c.line(16, 56, 34, 30, 5), c.line(34, 30, 52, 30, 5), c.poly([(52,24),(64,30),(52,36)])))
save("interpolate", 80, lambda c: (c.line(18, 60, 34, 40, 4), c.line(34, 40, 48, 48, 4), c.line(48, 48, 62, 24, 4), [c.disc(x, y, 5) for x, y in [(18, 60), (34, 40), (48, 48), (62, 24)]]))
# --- типы каналов (120×120, читаемее при мелком размере) ---
def audio(c):
    c.poly([(24, 46), (38, 46), (54, 32), (54, 62), (38, 48), (24, 48)])
    c.ring(60, 47, 10, 4, -55, 55)
    c.ring(60, 47, 18, 4, -45, 45)
save("audio", 120, audio)
def instrument(c):
    c.rect_o(16, 20, 64, 62, 4)
    for x in (26, 40, 54):
        c.line(x, 20, x, 46, 6)
save("instrument", 120, instrument)

print("создано:", len(os.listdir(OUT)), "иконок →", OUT)
