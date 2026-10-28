# Planche-contact : les captures <prefixe>_<n>.png de <debut> à <fin>,
# réduites, en grille, avec leur numéro. Usage : sheet.ps1 dossier prefixe debut fin pas sortie
param($dir, $prefix, [int]$from, [int]$to, [int]$step, $out)
Add-Type -AssemblyName System.Drawing
$frames = @()
for ($f = $from; $f -le $to; $f += $step) { $p = Join-Path $dir "${prefix}_$f.png"; if (Test-Path $p) { $frames += ,@($f, $p) } }
$cols = 4; $tw = 400; $th = 225
$rows = [Math]::Ceiling($frames.Count / $cols)
$sheet = New-Object System.Drawing.Bitmap ($cols * $tw), ($rows * $th)
$g = [System.Drawing.Graphics]::FromImage($sheet)
$font = New-Object System.Drawing.Font "Arial", 14
for ($i = 0; $i -lt $frames.Count; $i++) {
    $img = [System.Drawing.Image]::FromFile($frames[$i][1])
    $x = ($i % $cols) * $tw; $y = [Math]::Floor($i / $cols) * $th
    $g.DrawImage($img, $x, $y, $tw, $th)
    $g.DrawString([string]$frames[$i][0], $font, [System.Drawing.Brushes]::Yellow, $x + 4, $y + 4)
    $img.Dispose()
}
$sheet.Save($out); $g.Dispose(); $sheet.Dispose()
