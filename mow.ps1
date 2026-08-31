<#
    `mow.ps1` — bản PowerShell của `./mow`, cùng bộ lệnh.

        .\mow.ps1 up                 dựng toolbox và bật nó
        .\mow.ps1 shell              vào trong toolbox
        .\mow.ps1 test [đối số...]   chạy test
        .\mow.ps1 build              build workspace Rust
        .\mow.ps1 lint               fmt + clippy
        .\mow.ps1 exec <lệnh...>     chạy một lệnh bất kỳ bên trong
        .\mow.ps1 app up|down|logs   frontend + sidecar nhận thức
        .\mow.ps1 infra up|down      hạ tầng server mode
        .\mow.ps1 ai up|down|logs    máy chủ embedding cục bộ (TEI, cần GPU)
        .\mow.ps1 down               tắt
        .\mow.ps1 reset              tắt và xóa cả volume
        .\mow.ps1 doctor             kiểm tra máy thật
        .\mow.ps1 native <lệnh...>   chạy thẳng trên máy thật

    Repo phát triển chủ yếu trên Windows, nên bản này không phải thứ yếu —
    nó là đường đi mặc định ở đây.
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string] $Command = 'help',

    [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
    [string[]] $Rest = @()
)

$ErrorActionPreference = 'Stop'

$RepoRoot    = Split-Path -Parent $MyInvocation.MyCommand.Path
$ComposeFile = Join-Path $RepoRoot 'src\deploy\compose\docker-compose.yml'
$Service     = 'toolbox'

function Say  { param([string]$m) Write-Host "> $m" -ForegroundColor Green }
function Warn { param([string]$m) Write-Host "! $m" -ForegroundColor Yellow }
function Die  { param([string]$m) Write-Host "x $m" -ForegroundColor Red; exit 1 }

function Invoke-Compose {
    param([string[]]$Args)
    & docker compose -f $ComposeFile @Args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

function Assert-Docker {
    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        Die 'không tìm thấy `docker`. Cài Docker Desktop, hoặc dùng `.\mow.ps1 native <lệnh>`.'
    }
    & docker info *> $null
    if ($LASTEXITCODE -ne 0) {
        Die 'Docker có cài nhưng daemon không chạy. Bật Docker Desktop rồi thử lại.'
    }
}

function Assert-Up {
    Assert-Docker
    $id = (& docker compose -f $ComposeFile ps -q $Service 2>$null)
    if ([string]::IsNullOrWhiteSpace($id)) {
        Say 'toolbox chưa chạy, đang bật...'
        Invoke-Compose @('up', '-d', '--build', $Service)
    }
}

function Invoke-InBox {
    param([string]$Line)
    Assert-Up
    Invoke-Compose @('exec', '-T', $Service, 'bash', '-lc', $Line)
}

switch ($Command) {
    'up' {
        Assert-Docker
        Say 'dựng toolbox (lần đầu sẽ lâu — nó tải cả Rust, Node và uv)'
        Invoke-Compose @('up', '-d', '--build', $Service)
        Say 'cài phụ thuộc'
        Invoke-InBox 'cargo fetch --locked || cargo fetch'
        Say 'sẵn sàng. `.\mow.ps1 shell` để vào, `.\mow.ps1 test` để chạy test.'
    }

    'down' {
        Assert-Docker
        Invoke-Compose @('--profile', 'infra', 'down')
        Say 'đã tắt (volume vẫn còn — dùng `reset` nếu muốn xóa)'
    }

    'reset' {
        Assert-Docker
        Warn 'sẽ xóa toàn bộ volume: cache build và dữ liệu Postgres/Qdrant của môi trường test.'
        $ans = Read-Host "Gõ 'xoa' để xác nhận"
        if ($ans -ne 'xoa') { Die 'hủy' }
        Invoke-Compose @('--profile', 'infra', 'down', '-v')
        Say 'đã xóa sạch'
    }

    'shell' {
        Assert-Up
        Invoke-Compose @('exec', $Service, 'bash')
    }

    'exec' {
        if ($Rest.Count -eq 0) { Die 'cần một lệnh: .\mow.ps1 exec cargo tree' }
        Invoke-InBox ($Rest -join ' ')
    }

    'build' { Invoke-InBox ("cargo build --workspace " + ($Rest -join ' ')) }

    'test' {
        if ($Rest.Count -gt 0) { Invoke-InBox ("cargo test " + ($Rest -join ' ')) }
        else                   { Invoke-InBox 'cargo test --workspace --all-features' }
    }

    'lint' {
        Say 'fmt';    Invoke-InBox 'cargo fmt --all -- --check'
        Say 'clippy'; Invoke-InBox 'cargo clippy --workspace --all-targets --all-features -- -D warnings'
    }

    'fmt' { Invoke-InBox 'cargo fmt --all' }

    'determinism' {
        Assert-Up
        foreach ($n in 1, 2, 8) {
            Say "chạy với RAYON_NUM_THREADS=$n"
            Invoke-Compose @('exec', '-T', '-e', "RAYON_NUM_THREADS=$n", $Service,
                             'bash', '-lc', 'cargo test --workspace determinism -- --nocapture')
        }
    }

    'infra' {
        Assert-Docker
        $sub = if ($Rest.Count -gt 0) { $Rest[0] } else { 'up' }
        switch ($sub) {
            'up' {
                Say 'bật Postgres, NATS, Qdrant, Jaeger, MinIO'
                Invoke-Compose @('--profile', 'infra', 'up', '-d', 'postgres', 'nats', 'qdrant', 'jaeger', 'minio')
                Say 'Jaeger UI:  http://localhost:16686'
                Say 'Qdrant UI:  http://localhost:16333/dashboard'
                Say 'MinIO UI:   http://localhost:19001'
            }
            'down' { Invoke-Compose @('--profile', 'infra', 'stop', 'postgres', 'nats', 'qdrant', 'jaeger', 'minio') }
            default { Die 'infra up | infra down' }
        }
    }

    'logs' {
        Assert-Docker
        $svc = if ($Rest.Count -gt 0) { $Rest[0] } else { '' }
        if ($svc) { Invoke-Compose @('--profile', 'infra', 'logs', '-f', $svc) }
        else      { Invoke-Compose @('--profile', 'infra', 'logs', '-f') }
    }

    'app' {
        Assert-Docker
        $sub = if ($Rest.Count -gt 0) { $Rest[0] } else { 'up' }
        switch ($sub) {
            'up' {
                Say 'bat web + agent (lan dau phai cai phu thuoc, vai phut)'
                Invoke-Compose @('--profile', 'app', 'up', '-d', 'web', 'agent')
                Say 'Web:   http://localhost:15173'
                Say 'Agent: http://localhost:18765/health'
            }
            'down' { Invoke-Compose @('--profile', 'app', 'stop', 'web', 'agent') }
            'logs' { Invoke-Compose @('--profile', 'app', 'logs', '-f', 'web', 'agent') }
            default { Die 'app up | app down | app logs' }
        }
    }

    'ai' {
        Assert-Docker
        $sub = if ($Rest.Count -gt 0) { $Rest[0] } else { 'up' }
        switch ($sub) {
            'up' {
                Say 'bat may chu embedding (lan dau phai tai model, vai phut)'
                Invoke-Compose @('--profile', 'ai', 'up', '-d', 'embeddings')
                Say "san sang khi health xanh: http://localhost:18080"
            }
            'down' { Invoke-Compose @('--profile', 'ai', 'stop', 'embeddings') }
            'logs' { Invoke-Compose @('--profile', 'ai', 'logs', '-f', 'embeddings') }
            default { Die 'ai up | ai down | ai logs' }
        }
    }

    'ps' { Assert-Docker; Invoke-Compose @('--profile', 'infra', '--profile', 'ai', 'ps') }

    'native' {
        if ($Rest.Count -eq 0) { Die 'cần một lệnh: .\mow.ps1 native cargo test -p mow-math' }
        Push-Location (Join-Path $RepoRoot 'src')
        try { & $Rest[0] @($Rest[1..($Rest.Count - 1)]) }
        finally { Pop-Location }
    }

    'doctor' {
        Write-Host 'Kiểm tra môi trường' -ForegroundColor DarkGray
        $missing = 0
        foreach ($tool in 'docker', 'cargo', 'rustc', 'python', 'node', 'pnpm', 'uv', 'git') {
            $cmd = Get-Command $tool -ErrorAction SilentlyContinue
            if ($cmd) {
                $v = (& $tool --version 2>&1 | Select-Object -First 1)
                Write-Host ("  [ok]   {0,-8} {1}" -f $tool, $v) -ForegroundColor Green
            } else {
                Write-Host ("  [--]   {0,-8} không có" -f $tool) -ForegroundColor Yellow
                $missing++
            }
        }
        & docker info *> $null
        if ($LASTEXITCODE -eq 0) { Write-Host '  [ok]   docker daemon đang chạy' -ForegroundColor Green }
        else { Write-Host '  [--]   docker daemon không chạy — chỉ dùng được `native`' -ForegroundColor Yellow }
        if ($missing -eq 0) { Say 'đủ để chạy mọi thứ trên máy thật' }
        else { Warn 'thiếu vài thứ trên máy thật; container vẫn chạy đủ' }
    }

    default {
        Get-Help $MyInvocation.MyCommand.Path -Detailed | Out-String | Write-Host
    }
}
