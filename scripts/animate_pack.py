#!/usr/bin/env python3
"""为素材包生成 2 帧 APNG 部件动画（仍用 .png 扩展名，静态查看器看第一帧）。

用法（需要 Pillow：uv run --with pillow python scripts/animate_pack.py ...）：
  python scripts/animate_pack.py <素材包目录> gen        # 生成 APNG + B 帧预览
  python scripts/animate_pack.py <素材包目录> finalize   # 校验帧、写 animated/sha256、重打 fpack

gen 按 RECIPES 逐状态处理：连通块分离（猫主体=最大块，<64px 残块并入主体，
底部扁平块视为阴影不动），第 B 帧 = 主体压扁/跳起 + 配件（气泡/汗滴/Zzz/闪光）
位移；位移量按 图宽/1024 缩放，保证不同分辨率源图显示幅度一致。
happy/interacting 复用 celebrating、watching 复用 idle。
"""
import glob
import hashlib
import json
import os
import shutil
import sys
import zipfile
from collections import deque
from PIL import Image

DIR = sys.argv[1]
MODE = sys.argv[2]
FPACK = DIR.rstrip('/') + '.fpack'

RECIPES = {
    'idle':        {'squash': 0.97, 'dur': [700, 500]},
    'working':     {'squash': 0.98, 'dur': [450, 350]},
    'thinking':    {'squash': 0.97, 'acc': (0, -24), 'dur': [700, 500]},
    'celebrating': {'main': (0, -24), 'sparkle': True, 'dur': [380, 380]},
    'alarmed':     {'acc': (0, 28), 'dur': [450, 320]},
    'sleeping':    {'squash': 0.97, 'acc': (0, -20), 'dur': [900, 700]},
}


def label_components(px, w, h):
    label = [[-1] * w for _ in range(h)]
    comps = []
    for sy in range(h):
        for sx in range(w):
            if px[sx, sy][3] == 0 or label[sy][sx] != -1:
                continue
            cid = len(comps)
            seen = []
            dq = deque([(sx, sy)])
            label[sy][sx] = cid
            while dq:
                x, y = dq.popleft()
                seen.append((x, y))
                for dx in (-1, 0, 1):
                    for dy in (-1, 0, 1):
                        nx, ny = x + dx, y + dy
                        if 0 <= nx < w and 0 <= ny < h and px[nx, ny][3] > 0 \
                                and label[ny][nx] == -1:
                            label[ny][nx] = cid
                            dq.append((nx, ny))
            xs = [p[0] for p in seen]
            ys = [p[1] for p in seen]
            comps.append({'pix': seen,
                          'bbox': (min(xs), min(ys), max(xs), max(ys))})
    return comps


def is_shadow(comp, h):
    x0, y0, x1, y1 = comp['bbox']
    return y1 >= 0.88 * h and (y1 - y0) <= 0.18 * h


def comp_image(px, comp):
    x0, y0, x1, y1 = comp['bbox']
    sub = Image.new('RGBA', (x1 - x0 + 1, y1 - y0 + 1), (0, 0, 0, 0))
    sp = sub.load()
    for (x, y) in comp['pix']:
        sp[x - x0, y - y0] = px[x, y]
    return sub


def squash_bottom(img, factor):
    w, h = img.size
    nh = max(1, int(h * factor))
    small = img.resize((w, nh), Image.NEAREST)
    out = Image.new('RGBA', (w, h), (0, 0, 0, 0))
    out.paste(small, (0, h - nh), small)
    return out


def build_frame_b(base, comps, recipe):
    main = max(comps, key=lambda c: len(c['pix']))
    px = base.load()
    s = base.size[0] / 1024.0
    frame = Image.new('RGBA', base.size, (0, 0, 0, 0))
    for c in sorted(comps, key=lambda c: -len(c['pix'])):
        sub = comp_image(px, c)
        x0, y0 = c['bbox'][0], c['bbox'][1]
        dx = dy = 0
        if c is main:
            if 'squash' in recipe:
                sub = squash_bottom(sub, recipe['squash'])
            if 'main' in recipe:
                dx, dy = (int(recipe['main'][0] * s), int(recipe['main'][1] * s))
        elif not is_shadow(c, base.size[1]):
            if 'acc' in recipe:
                dx, dy = (int(recipe['acc'][0] * s), int(recipe['acc'][1] * s))
            elif recipe.get('sparkle'):
                cx = (c['bbox'][0] + c['bbox'][2]) / 2
                dx = int(8 * s) if cx >= base.size[0] / 2 else -int(8 * s)
                dy = -int(12 * s)
        frame.paste(sub, (x0 + dx, y0 + dy), sub)
    return frame


def gen():
    for state, recipe in RECIPES.items():
        path = os.path.join(DIR, state + '.png')
        if not os.path.exists(path):
            print(state, 'missing, skip')
            continue
        base = Image.open(path).convert('RGBA')
        w, h = base.size
        px = base.load()
        comps = label_components(px, w, h)
        main0 = max(comps, key=lambda c: len(c['pix']))
        kept = []
        for c in comps:
            if c is not main0 and len(c['pix']) < 64:
                main0['pix'].extend(c['pix'])
            else:
                kept.append(c)
        xs = [p[0] for p in main0['pix']]
        ys = [p[1] for p in main0['pix']]
        main0['bbox'] = (min(xs), min(ys), max(xs), max(ys))
        comps = kept
        info = []
        for c in comps:
            info.append((len(c['pix']), c['bbox'],
                         'shadow' if (c is not main0 and is_shadow(c, h)) else ''))
        print(state, 'comps:', info)
        frame_b = build_frame_b(base, comps, recipe)
        base.save(path, save_all=True, append_images=[frame_b],
                  duration=recipe['dur'], loop=0, disposal=2, blend=0)
        frame_b.save(os.path.join(DIR, '.tmp_b_%s.png' % state))
        print(state, 'apng saved')
    for src, dst in (('celebrating', 'happy'), ('celebrating', 'interacting'),
                     ('idle', 'watching')):
        s = os.path.join(DIR, src + '.png')
        if os.path.exists(s):
            shutil.copyfile(s, os.path.join(DIR, dst + '.png'))
    print('copies done')


def finalize():
    for p in sorted(glob.glob(os.path.join(DIR, '*.png'))):
        im = Image.open(p)
        print(os.path.basename(p), 'frames:', getattr(im, 'n_frames', 1),
              'dur:', im.info.get('duration'))
    for p in glob.glob(os.path.join(DIR, '.tmp_b_*.png')):
        os.remove(p)
    pngs = sorted(glob.glob(os.path.join(DIR, '*.png')))
    sha = hashlib.sha256()
    for p in pngs:
        sha.update(open(p, 'rb').read())
    pack_path = os.path.join(DIR, 'pack.json')
    pack = json.load(open(pack_path, encoding='utf-8'))
    pack['animated'] = True
    pack['sha256'] = sha.hexdigest()
    with open(pack_path, 'w', encoding='utf-8') as f:
        json.dump(pack, f, ensure_ascii=False, indent=2)
        f.write('\n')
    with zipfile.ZipFile(FPACK, 'r') as old:
        extras = {n: old.read(n) for n in old.namelist()
                  if not n.endswith('.png') and n != 'pack.json'}
    with zipfile.ZipFile(FPACK, 'w', zipfile.ZIP_DEFLATED) as z:
        z.writestr('pack.json', open(pack_path, 'rb').read())
        for name, data in extras.items():
            z.writestr(name, data)
        for p in pngs:
            z.write(p, os.path.basename(p))
    print('sha256/fpack updated', sha.hexdigest())


if __name__ == '__main__':
    gen() if MODE == 'gen' else finalize()
