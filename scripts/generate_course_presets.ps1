Add-Type -AssemblyName System.Drawing

$outDir = Join-Path (Get-Location) "frontend/static/images/course-presets"
New-Item -ItemType Directory -Force $outDir | Out-Null

$width = 1600
$height = 900

$presets = @(
    @{ File="software-development.jpg"; A="#14213d"; B="#00b4d8"; Accent="#fca311"; Motif="code" },
    @{ File="data-analytics.jpg"; A="#123524"; B="#3a86ff"; Accent="#95d5b2"; Motif="chart" },
    @{ File="cybersecurity.jpg"; A="#111827"; B="#0f766e"; Accent="#22c55e"; Motif="shield" },
    @{ File="cloud-computing.jpg"; A="#0f172a"; B="#38bdf8"; Accent="#e0f2fe"; Motif="cloud" },
    @{ File="artificial-intelligence.jpg"; A="#2d1b69"; B="#06b6d4"; Accent="#f0abfc"; Motif="nodes" },
    @{ File="business-management.jpg"; A="#1f2937"; B="#64748b"; Accent="#facc15"; Motif="blocks" },
    @{ File="digital-marketing.jpg"; A="#7c2d12"; B="#db2777"; Accent="#fed7aa"; Motif="megaphone" },
    @{ File="entrepreneurship.jpg"; A="#064e3b"; B="#f97316"; Accent="#fde68a"; Motif="arrow" },
    @{ File="finance.jpg"; A="#0f172a"; B="#15803d"; Accent="#bbf7d0"; Motif="lines" },
    @{ File="project-management.jpg"; A="#312e81"; B="#475569"; Accent="#c4b5fd"; Motif="timeline" },
    @{ File="design-thinking.jpg"; A="#831843"; B="#f59e0b"; Accent="#fbcfe8"; Motif="spark" },
    @{ File="ui-ux-design.jpg"; A="#1e1b4b"; B="#14b8a6"; Accent="#a5f3fc"; Motif="frames" },
    @{ File="photography.jpg"; A="#18181b"; B="#78716c"; Accent="#f5f5f4"; Motif="lens" },
    @{ File="healthcare.jpg"; A="#164e63"; B="#22c55e"; Accent="#cffafe"; Motif="cross" },
    @{ File="education.jpg"; A="#1d4ed8"; B="#7c3aed"; Accent="#dbeafe"; Motif="book" },
    @{ File="communication.jpg"; A="#4c1d95"; B="#e11d48"; Accent="#fce7f3"; Motif="chat" },
    @{ File="leadership.jpg"; A="#0f172a"; B="#b45309"; Accent="#fde68a"; Motif="peak" },
    @{ File="languages.jpg"; A="#155e75"; B="#9333ea"; Accent="#f0f9ff"; Motif="bubbles" },
    @{ File="engineering.jpg"; A="#27272a"; B="#2563eb"; Accent="#e5e7eb"; Motif="grid" },
    @{ File="hospitality.jpg"; A="#713f12"; B="#dc2626"; Accent="#fef3c7"; Motif="waves" }
)

function ColorFromHex([string]$hex) {
    return [System.Drawing.ColorTranslator]::FromHtml($hex)
}

function Add-TranslucentBrush([string]$hex, [int]$alpha) {
    $c = ColorFromHex $hex
    return New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb($alpha, $c.R, $c.G, $c.B))
}

function Add-TranslucentPen([string]$hex, [int]$alpha, [float]$size) {
    $c = ColorFromHex $hex
    $color = [System.Drawing.Color]::FromArgb($alpha, $c.R, $c.G, $c.B)
    return New-Object System.Drawing.Pen -ArgumentList $color, $size
}

function Fill-RoundedRect($g, $brush, [int]$x, [int]$y, [int]$w, [int]$h, [int]$r) {
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    $g.FillPath($brush, $path)
    $path.Dispose()
}

function Draw-Motif($g, $preset) {
    $accent = $preset.Accent
    $brush = Add-TranslucentBrush $accent 72
    $brushStrong = Add-TranslucentBrush $accent 118
    $pen = Add-TranslucentPen $accent 150 8
    $penThin = Add-TranslucentPen $accent 90 3

    switch ($preset.Motif) {
        "code" {
            for ($i = 0; $i -lt 8; $i++) {
                $y = 170 + ($i * 70)
                $g.DrawLine($penThin, 180, $y, 700 + ($i * 55), $y)
                $g.FillEllipse($brushStrong, 120, $y - 10, 20, 20)
            }
            $g.DrawString("{ }", (New-Object System.Drawing.Font("Consolas", 150, [System.Drawing.FontStyle]::Bold)), $brush, 1010, 315)
        }
        "chart" {
            $points = @(
                [System.Drawing.Point]::new(180, 650), [System.Drawing.Point]::new(390, 520),
                [System.Drawing.Point]::new(610, 570), [System.Drawing.Point]::new(820, 360),
                [System.Drawing.Point]::new(1120, 430), [System.Drawing.Point]::new(1370, 230)
            )
            $g.DrawLines($pen, $points)
            foreach ($p in $points) { $g.FillEllipse($brushStrong, $p.X - 16, $p.Y - 16, 32, 32) }
        }
        "shield" {
            $poly = @([System.Drawing.Point]::new(800,170),[System.Drawing.Point]::new(1110,280),[System.Drawing.Point]::new(1050,620),[System.Drawing.Point]::new(800,760),[System.Drawing.Point]::new(550,620),[System.Drawing.Point]::new(490,280))
            $g.FillPolygon($brush, $poly)
            $g.DrawPolygon($pen, $poly)
        }
        "cloud" {
            $g.FillEllipse($brush, 410, 360, 360, 210)
            $g.FillEllipse($brush, 650, 280, 380, 280)
            $g.FillEllipse($brush, 920, 360, 320, 220)
            $g.FillRectangle($brush, 560, 450, 600, 150)
        }
        "nodes" {
            $nodes = @([System.Drawing.Point]::new(440,250),[System.Drawing.Point]::new(760,180),[System.Drawing.Point]::new(1080,290),[System.Drawing.Point]::new(610,560),[System.Drawing.Point]::new(970,620),[System.Drawing.Point]::new(1260,470))
            for ($i = 0; $i -lt $nodes.Count; $i++) { for ($j = $i + 1; $j -lt $nodes.Count; $j++) { if (($i + $j) % 2 -eq 0) { $g.DrawLine($penThin, $nodes[$i], $nodes[$j]) } } }
            foreach ($p in $nodes) { $g.FillEllipse($brushStrong, $p.X - 34, $p.Y - 34, 68, 68) }
        }
        "blocks" {
            for ($i = 0; $i -lt 5; $i++) { $g.FillRectangle($brush, 320 + $i * 190, 250 + ($i % 2) * 90, 140, 260) }
        }
        "megaphone" {
            $g.FillPolygon($brush, @([System.Drawing.Point]::new(420,430),[System.Drawing.Point]::new(1060,230),[System.Drawing.Point]::new(1060,650)))
            $g.FillRectangle($brushStrong, 300, 380, 220, 120)
            $g.DrawArc($pen, 1050, 270, 230, 320, -55, 110)
        }
        "arrow" {
            $g.DrawLine($pen, 260, 650, 1220, 260)
            $g.FillPolygon($brushStrong, @([System.Drawing.Point]::new(1220,260),[System.Drawing.Point]::new(1110,230),[System.Drawing.Point]::new(1160,350)))
        }
        "lines" {
            for ($i = 0; $i -lt 10; $i++) { $g.DrawLine($penThin, 180, 220 + $i * 55, 1380, 120 + $i * 60) }
        }
        "timeline" {
            $g.DrawLine($pen, 260, 450, 1340, 450)
            foreach ($x in @(320, 570, 820, 1070, 1320)) { $g.FillEllipse($brushStrong, $x - 28, 422, 56, 56) }
        }
        "spark" {
            for ($i = 0; $i -lt 12; $i++) {
                $angle = ($i * 30) * [Math]::PI / 180
                $g.DrawLine($pen, 800, 450, 800 + [Math]::Cos($angle) * 360, 450 + [Math]::Sin($angle) * 260)
            }
            $g.FillEllipse($brushStrong, 730, 380, 140, 140)
        }
        "frames" {
            for ($i = 0; $i -lt 4; $i++) { $g.DrawRectangle($penThin, 280 + $i * 210, 230 + $i * 50, 360, 240) }
        }
        "lens" {
            $g.FillEllipse($brush, 570, 220, 460, 460)
            $g.DrawEllipse($pen, 570, 220, 460, 460)
            $g.FillEllipse($brushStrong, 705, 355, 190, 190)
        }
        "cross" {
            $g.FillRectangle($brush, 700, 240, 200, 420)
            $g.FillRectangle($brush, 590, 350, 420, 200)
        }
        "book" {
            $g.FillPie($brush, 330, 250, 470, 420, 270, 180)
            $g.FillPie($brush, 800, 250, 470, 420, 90, 180)
            $g.DrawLine($pen, 800, 270, 800, 690)
        }
        "chat" {
            Fill-RoundedRect $g $brush 310 270 440 260 34
            Fill-RoundedRect $g $brushStrong 790 390 500 270 34
        }
        "peak" {
            $g.FillPolygon($brush, @([System.Drawing.Point]::new(250,690),[System.Drawing.Point]::new(650,280),[System.Drawing.Point]::new(920,690)))
            $g.FillPolygon($brushStrong, @([System.Drawing.Point]::new(670,690),[System.Drawing.Point]::new(1070,180),[System.Drawing.Point]::new(1370,690)))
        }
        "bubbles" {
            foreach ($c in @(@(300,330,170),@(550,520,120),@(780,290,210),@(1060,500,170),@(1280,260,120))) {
                $g.FillEllipse($brush, $c[0], $c[1], $c[2], $c[2])
                $g.DrawEllipse($penThin, $c[0], $c[1], $c[2], $c[2])
            }
        }
        "grid" {
            for ($x = 170; $x -lt 1450; $x += 120) { $g.DrawLine($penThin, $x, 160, $x, 740) }
            for ($y = 170; $y -lt 740; $y += 90) { $g.DrawLine($penThin, 170, $y, 1450, $y) }
        }
        "waves" {
            for ($i = 0; $i -lt 8; $i++) {
                $path = New-Object System.Drawing.Drawing2D.GraphicsPath
                $y = 250 + $i * 60
                $path.AddBezier(140, $y, 420, $y - 90, 680, $y + 90, 960, $y)
                $path.AddBezier(960, $y, 1120, $y - 70, 1320, $y + 70, 1480, $y)
                $g.DrawPath($penThin, $path)
                $path.Dispose()
            }
        }
    }

    $brush.Dispose()
    $brushStrong.Dispose()
    $pen.Dispose()
    $penThin.Dispose()
}

foreach ($preset in $presets) {
    $bitmap = New-Object System.Drawing.Bitmap $width, $height
    $g = [System.Drawing.Graphics]::FromImage($bitmap)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

    $rect = New-Object System.Drawing.Rectangle 0, 0, $width, $height
    $gradient = New-Object System.Drawing.Drawing2D.LinearGradientBrush $rect, (ColorFromHex $preset.A), (ColorFromHex $preset.B), 35
    $g.FillRectangle($gradient, $rect)
    $gradient.Dispose()

    $overlay = Add-TranslucentBrush "#ffffff" 18
    for ($i = 0; $i -lt 8; $i++) {
        $g.FillEllipse($overlay, -120 + ($i * 250), 640 - ($i % 3) * 120, 340, 340)
    }
    $overlay.Dispose()

    Draw-Motif $g $preset

    $vignette = New-Object System.Drawing.Drawing2D.GraphicsPath
    $vignette.AddRectangle($rect)
    $dark = Add-TranslucentBrush "#000000" 40
    $g.FillPath($dark, $vignette)
    $dark.Dispose()
    $vignette.Dispose()

    $codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq "image/jpeg" }
    $params = New-Object System.Drawing.Imaging.EncoderParameters 1
    $params.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), 92L
    $bitmap.Save((Join-Path $outDir $preset.File), $codec, $params)

    $params.Dispose()
    $g.Dispose()
    $bitmap.Dispose()
}
