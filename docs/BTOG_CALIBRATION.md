# Tox d’étalonnage BtogV (geometry `*.data`)

Pour que le parseur OpenTouchDesigner corresponde à TouchDesigner, il faut des fichiers dont **on connaît la géométrie réelle**. Le théière et le protomarquee sont trop gros pour débugger à la main.

## Ce qu’il faut fabriquer dans TD

Un seul COMP (ex. `CALIB_base`) avec **quatre branches**, chacune avec **Viewer** actif :

| Node | But |
|------|-----|
| `box1` | **Box SOP** — Size `1, 1, 1`, Centré à l’origine. Une boîte unitaire = on connaît les bornes `[-0.5, 0.5]`. |
| `grid1` | **Grid SOP** — Rows/Columns `2, 2`, une feuille plane (4 verts, 2 tris) = indices simples. |
| `triangle` / `filein2` / `filein_tri` | **File In SOP** → `calib_tri.obj` (voir repo `examples/calib_tri.obj`). |
| `filein1` (ou `filein_builtin`) | **File In SOP** avec `app.samplesFolder + '/Geo/TeaPot.tog'` (ou `datto1.tog`) — référence `.tog` natif. |

### Triangle OBJ de référence (obligatoire)

Copie ou crée `calib_tri.obj` à côté du tox (modèle dans `examples/calib_tri.obj`) :

```obj
# Un triangle dans le plan XY, facile à reconnaître dans un viewer
v 0 0 0
v 2 0 0
v 0 3 0
f 1 2 3
```

Dans TD : **File In SOP** → `calib_tri.obj`, **Lock** actif sur chaque SOP d’étalonnage. Sauvegarde le tox : la géométrie cuisinée est dans le sidecar `*.data` **embarqué** (pas le fichier OBJ en bout de chaîne).

Côté OTD, on ne lit que ces **`.data` lockés** à l’import ; `calib_tri.obj` sert uniquement de **référence** pour comparer les dumps.

Petits sidecars lockés (`triangle`, `grid1`, …) : entête octet **16 = 1**, positions en **float16** aux offsets `14 + 4·index` dans chaque record 28 octets (pas le float32 @(0,4,8) de la box).

### DAT texte (très utile)

Un **Text DAT** nommé `CALIB_readme` avec le contenu exact attendu, par exemple :

```
box1: AABB about [-0.5,0.5] on each axis, 8 points (or 24 if subdivided)
grid1: 4 points, 2 triangles, Z=0
filein_tri: vertices (0,0,0) (2,0,0) (0,3,0)
```

Comme ça, l’import garde la spec dans le tox.

## Ce qu’il faut envoyer / mettre dans le repo

1. `calib.tox` **ou** le dossier développé `calib.tox.dir/` **complet** (tous les `.data`, `.n`, `.parm`).
2. Le fichier `calib_tri.obj` (s’il n’est pas déjà dans le tox).
3. Optionnel : une capture TD du viewer pour `filein_tri` (référence visuelle).

## Côté OTD : dump du parseur

Après avoir placé les fichiers dans le repo :

```bash
cargo test -p otd-sop loads_teapot -q
cargo test -p otd-td teapot_ -q
cargo run -p otd-cli -- btog-dump teapot.tox.dir/base1/triangle.data
cargo run -p otd-cli -- btog-dump teapot.tox.dir/base1/grid1.data
cargo run -p otd-cli -- btog-dump teapot.tox.dir/base1/filein1.data
```

Colle la sortie dans une issue / message : on compare **header, premiers points, premières primitives, triangles cuisinés** avec la spec du Text DAT.

## État du parseur (teapot tox)

| Sidecar | Attendu | Tests OTD |
|---------|---------|-----------|
| `box.data` | cube **108 tris**. `P` est la grille `x*16 + y*4 + z`. `N` du fichier est `normalize(P)` (un sommet partagé par 3 faces) : à l’affichage chaque face a sa propre copie de sommets et la normale de la face, sinon l’éclairage dégrade le plan. Ou 24 pts → fallback cube 12 tris. | `loads_teapot_box_*`, `teapot_box_cooks_*` |
| `triangle.data` | 3 pts, 1 tri ~2×3 | `loads_teapot_triangle_*`, `teapot_triangle_cooks_*` |
| `grid1.data` | 4 pts, 2 tris 2×2, Z=0 | `loads_teapot_grid_*`, `teapot_grid_cooks_*` |
| `filein1.data` | théière `.tog` : **1760 pts**, **1698 polys** = **1670 quads + 28 tris** → **3368 tris**. Un seul *run* GPD (`0xffffffff`, longueur 1698, clé polygone `1`). Chaque sommet : `uint16` + 12 octets d’attribut. `0x3c` est le caractère `'<'` (face fermée), pas un marqueur libre. | `teapot_filein_cooks_*` |

Les **micro** sidecars lockés (`data[16]==1`, triangle / grille) valident positions **et** indices. Les gros fichiers **tagged** (théière, protomarquee) reposent surtout sur le chemin float32 + marqueurs `3c 00` — à recouper avec TD sur les cas qui échouent encore.

## `.tog` (TouchDesigner) et BtogV (calib)

`.tog` est le `.geo` / `.bgeo` de PRISMS / Houdini (SideFX), renommé quand Derivative s’est séparé de SideFX. Le binaire est le format GPD **V5** : magic `Bgeo` + `'V'` + `int32` version, puis `NPoints`, `NPrims`, groupes et attributs, tout en **big-endian**. TouchDesigner écrit `Btog` à la place de `Bgeo`. L’ASCII équivalent commence par `PGEOMETRY V5`.

Spec : `$HH/public/GPD/gpd.txt` dans Houdini (l’ancien `.bhclassic`). Un polygone binaire est `PrimKey = 1`, `#vtx:int32`, `'<'` fermé ou `':'` ouvert, puis un `uint16` par sommet (si `NPoints ≤ 65535`). Plusieurs polygones d’affilée sont un `Run` (`0xffffffff`, longueur `u16`, puis la clé).

OpenTouchDesigner lit ce stockage sous le nom **BtogV** (sidecars `*.data` dans un `.tox`).

À l’import, le flag TD **`lock on`** devient `flags.lock` ; au cook, un SOP verrouillé charge d’abord le sidecar `nom.data` (BtogV) au lieu de régénérer (grille, boîte, etc.).

| Concept TD (`.tog`) | BtogV / locked `*.data` |
|---------------------|-------------------------|
| Point list (XYZ)    | Enregistrements float16 taggés (stride 28 depuis l’octet 75 ; micro `data[16]==1`) |
| Vertex / attributs  | Paires demi-float dans l’enregistrement point + flottants UV en queue |
| Primitive list      | Run GPD de polygones (`'<'` / `':'` + indices `uint16`). Les petits sidecars gardent aussi la table `00 10 00 00`. |
