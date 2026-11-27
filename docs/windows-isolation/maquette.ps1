# Maquette jetable d'ESC-591 (docs/WINDOWS-ISOLATION.md) : jamais branchée à l'app.
#
# Elle essaie deux mécanismes de cloisonnement de Windows sur des dossiers qu'elle crée elle-même
# dans un dossier temporaire, sans droit d'administrateur, puis défait tout ce qu'elle a posé :
#
#   R  un jeton restreint (CreateRestrictedToken) dont la liste de SID restrictifs porte un SID de
#      l'ensemble, et une ACE de refus pour ce SID sur le dossier « privé » ;
#   A  un AppContainer (SECURITY_CAPABILITIES) : son profil est créé le temps de l'essai, puis
#      retiré ; le dossier « public » seul lui est ouvert par une ACE ; aucune capacité réseau.
#
# Chaque essai lance `cmd.exe /c type` (lecture), `powershell`, une écriture, `git` et `curl.exe` vers un écouteur
# 127.0.0.1 de la maquette, et note ce qui passe. Port de l'écouteur : $env:ESCOUADE_PORT_BASE,
# sinon un port libre que donne le système.
#
# Ce qu'elle laisse : rien. Le dossier temporaire est retiré (ACL comprises), le profil de
# l'AppContainer aussi (DeleteAppContainerProfile, vérifié à la fin) ; aucun compte, aucune règle de
# pare-feu, aucun autre réglage du système.
#
# Usage : pwsh -NoProfile -File docs/windows-isolation/maquette.ps1

$ErrorActionPreference = 'Stop'
$port = if ($env:ESCOUADE_PORT_BASE) { [int]$env:ESCOUADE_PORT_BASE } else { 0 }

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Security.Principal;

public static class Maquette {
    [StructLayout(LayoutKind.Sequential)] struct SID_AND_ATTRIBUTES { public IntPtr Sid; public uint Attributes; }
    [StructLayout(LayoutKind.Sequential)] struct SECURITY_CAPABILITIES {
        public IntPtr AppContainerSid; public IntPtr Capabilities; public uint CapabilityCount; public uint Reserved; }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] struct STARTUPINFO {
        public int cb; public string lpReserved, lpDesktop, lpTitle;
        public int dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags;
        public short wShowWindow, cbReserved2; public IntPtr lpReserved2, hStdInput, hStdOutput, hStdError; }
    [StructLayout(LayoutKind.Sequential)] struct STARTUPINFOEX { public STARTUPINFO StartupInfo; public IntPtr lpAttributeList; }
    [StructLayout(LayoutKind.Sequential)] struct PROCESS_INFORMATION { public IntPtr hProcess, hThread; public int dwProcessId, dwThreadId; }

    [DllImport("advapi32.dll", SetLastError = true)] static extern bool OpenProcessToken(IntPtr p, uint access, out IntPtr token);
    [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
    [DllImport("advapi32.dll", SetLastError = true)] static extern bool CreateRestrictedToken(IntPtr existing, uint flags,
        uint disableCount, IntPtr disable, uint deleteCount, IntPtr delete, uint restrictCount, SID_AND_ATTRIBUTES[] restrict, out IntPtr token);
    [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)] static extern bool CreateProcessAsUser(IntPtr token,
        string app, string cmd, IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr env, string cwd, ref STARTUPINFO si, out PROCESS_INFORMATION pi);
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)] static extern bool CreateProcess(string app, string cmd,
        IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr env, string cwd, ref STARTUPINFOEX si, out PROCESS_INFORMATION pi);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool InitializeProcThreadAttributeList(IntPtr list, int count, int flags, ref IntPtr size);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool UpdateProcThreadAttribute(IntPtr list, uint flags, IntPtr attr,
        IntPtr value, IntPtr size, IntPtr prev, IntPtr ret);
    [DllImport("kernel32.dll")] static extern void DeleteProcThreadAttributeList(IntPtr list);
    [DllImport("kernel32.dll")] static extern uint WaitForSingleObject(IntPtr h, uint ms);
    [DllImport("kernel32.dll")] static extern bool GetExitCodeProcess(IntPtr h, out uint code);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
    [DllImport("userenv.dll", CharSet = CharSet.Unicode)] static extern int DeriveAppContainerSidFromAppContainerName(string name, out IntPtr sid);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool ConvertSidToStringSid(IntPtr sid, out IntPtr str);
    [DllImport("advapi32.dll")] static extern IntPtr FreeSid(IntPtr sid);
    [DllImport("userenv.dll", CharSet = CharSet.Unicode)] static extern int CreateAppContainerProfile(string name, string display,
        string description, IntPtr caps, uint count, out IntPtr sid);
    [DllImport("userenv.dll", CharSet = CharSet.Unicode)] static extern int DeleteAppContainerProfile(string name);
    [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr p);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool ConvertStringSecurityDescriptorToSecurityDescriptor(
        string sddl, uint rev, out IntPtr sd, IntPtr size);
    [DllImport("advapi32.dll", SetLastError = true)] static extern bool GetSecurityDescriptorDacl(IntPtr sd, out bool present, out IntPtr dacl, out bool defaulted);
    [DllImport("advapi32.dll", SetLastError = true)] static extern bool SetTokenInformation(IntPtr token, int cls, ref IntPtr info, uint len);

    const uint CREATE_NO_WINDOW = 0x08000000, EXTENDED_STARTUPINFO_PRESENT = 0x00080000;
    static readonly IntPtr SECURITY_CAPABILITIES_ATTR = (IntPtr)0x00020009;

    static IntPtr Sid(string s) { var b = new byte[new SecurityIdentifier(s).BinaryLength];
        new SecurityIdentifier(s).GetBinaryForm(b, 0); var p = Marshal.AllocHGlobal(b.Length); Marshal.Copy(b, 0, p, b.Length); return p; }

    static uint Wait(PROCESS_INFORMATION pi) { WaitForSingleObject(pi.hProcess, 30000); uint code; GetExitCodeProcess(pi.hProcess, out code);
        CloseHandle(pi.hProcess); CloseHandle(pi.hThread); return code; }

    // R : un jeton restreint du jeton de l'utilisateur. Ses SID restrictifs : ce que les ACL
    // ordinaires accordent (l'utilisateur, sa session, Tout le monde, Utilisateurs authentifiés,
    // Utilisateurs, Interactif, RESTRICTED), plus le SID de l'ensemble que refuse le dossier privé.
    [StructLayout(LayoutKind.Sequential)] struct TOKEN_MANDATORY_LABEL { public SID_AND_ATTRIBUTES Label; }
    [DllImport("advapi32.dll", SetLastError = true)] static extern bool SetTokenInformation(IntPtr token, int cls, ref TOKEN_MANDATORY_LABEL info, uint len);

    public static uint RunRestricted(string cmd, string cwd, string setSid) { return RunRestricted(cmd, cwd, setSid, true, false); }
    public static uint RunRestricted(string cmd, string cwd, string setSid, bool withUser) { return RunRestricted(cmd, cwd, setSid, withUser, false); }
    // low : le jeton passe en faible intégrité (S-1-16-4096) : il n'écrit plus dans un objet de
    // niveau moyen (un fichier, un processus) que s'il porte l'étiquette « bas ».
    public static uint RunRestricted(string cmd, string cwd, string setSid, bool withUser, bool low) {
        IntPtr token; if (!OpenProcessToken(GetCurrentProcess(), 0x00F01FF, out token)) throw new System.ComponentModel.Win32Exception();
        var me = WindowsIdentity.GetCurrent();
        var list = new System.Collections.Generic.List<string> { "S-1-1-0", "S-1-5-11", "S-1-5-32-545", "S-1-5-4", "S-1-5-12", setSid };
        if (withUser) list.Add(me.User.Value);
        // La session : sans elle, le bureau et la station refusent le processus (0xC0000142).
        foreach (var g in me.Groups) if (g.Value.StartsWith("S-1-5-5-")) list.Add(g.Value);
        var sids = new SID_AND_ATTRIBUTES[list.Count];
        for (int i = 0; i < list.Count; i++) sids[i] = new SID_AND_ATTRIBUTES { Sid = Sid(list[i]) };
        IntPtr restricted;
        if (!CreateRestrictedToken(token, 0, 0, IntPtr.Zero, 0, IntPtr.Zero, (uint)sids.Length, sids, out restricted))
            throw new Exception("CreateRestrictedToken : " + new System.ComponentModel.Win32Exception().Message);
        if (!withUser) DefaultDacl(restricted, me.User.Value, setSid);
        if (low) {
            var label = new TOKEN_MANDATORY_LABEL { Label = new SID_AND_ATTRIBUTES { Sid = Sid("S-1-16-4096"), Attributes = 0x20 } };
            if (!SetTokenInformation(restricted, 25, ref label, (uint)(Marshal.SizeOf(label) + 12)))
                throw new Exception("TokenIntegrityLevel : " + new System.ComponentModel.Win32Exception().Message);
        }
        var si = new STARTUPINFO(); si.cb = Marshal.SizeOf(si); PROCESS_INFORMATION pi;
        if (!CreateProcessAsUser(restricted, null, cmd, IntPtr.Zero, IntPtr.Zero, false, CREATE_NO_WINDOW, IntPtr.Zero, cwd, ref si, out pi))
            throw new Exception("CreateProcessAsUser : " + new System.ComponentModel.Win32Exception().Message);
        CloseHandle(restricted); CloseHandle(token);
        return Wait(pi);
    }

    // Le DACL par défaut du jeton (ce que reçoit chaque objet que le processus crée, lui compris) :
    // sans un SID restrictif dedans, le processus ne s'ouvre pas lui-même et meurt à son démarrage.
    static void DefaultDacl(IntPtr token, string user, string setSid) {
        IntPtr sd; if (!ConvertStringSecurityDescriptorToSecurityDescriptor("D:(A;;GA;;;SY)(A;;GA;;;" + user + ")(A;;GA;;;" + setSid + ")(A;;GA;;;RC)", 1, out sd, IntPtr.Zero))
            throw new System.ComponentModel.Win32Exception();
        bool present, defaulted; IntPtr dacl; GetSecurityDescriptorDacl(sd, out present, out dacl, out defaulted);
        if (!SetTokenInformation(token, 6, ref dacl, (uint)IntPtr.Size)) throw new Exception("SetTokenInformation : " + new System.ComponentModel.Win32Exception().Message);
    }

    public static string AppContainerSid(string name) {
        IntPtr sid; int hr = DeriveAppContainerSidFromAppContainerName(name, out sid); if (hr != 0) throw new Exception("hr " + hr);
        IntPtr s; ConvertSidToStringSid(sid, out s); var str = Marshal.PtrToStringUni(s); LocalFree(s); FreeSid(sid); return str;
    }

    // Le profil d'un AppContainer (clé sous HKCU et dossier sous %LOCALAPPDATA%\Packages) : sans lui,
    // CreateProcess répond « fichier introuvable ». La maquette le crée, puis le retire.
    public static int CreateProfile(string name) { IntPtr sid; int hr = CreateAppContainerProfile(name, name, name, IntPtr.Zero, 0, out sid);
        if (hr == 0) FreeSid(sid); return hr; }
    public static int DeleteProfile(string name) { return DeleteAppContainerProfile(name); }

    // A : un AppContainer, sans capacité.
    public static uint RunAppContainer(string cmd, string cwd, string containerSid) {
        var caps = new SECURITY_CAPABILITIES { AppContainerSid = Sid(containerSid) };
        IntPtr capsPtr = Marshal.AllocHGlobal(Marshal.SizeOf(caps)); Marshal.StructureToPtr(caps, capsPtr, false);
        IntPtr size = IntPtr.Zero; InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref size);
        IntPtr attrs = Marshal.AllocHGlobal(size);
        if (!InitializeProcThreadAttributeList(attrs, 1, 0, ref size)) throw new System.ComponentModel.Win32Exception();
        if (!UpdateProcThreadAttribute(attrs, 0, SECURITY_CAPABILITIES_ATTR, capsPtr, (IntPtr)Marshal.SizeOf(caps), IntPtr.Zero, IntPtr.Zero))
            throw new System.ComponentModel.Win32Exception();
        var si = new STARTUPINFOEX(); si.StartupInfo.cb = Marshal.SizeOf(si); si.lpAttributeList = attrs; PROCESS_INFORMATION pi;
        if (!CreateProcess(null, cmd, IntPtr.Zero, IntPtr.Zero, false, CREATE_NO_WINDOW | EXTENDED_STARTUPINFO_PRESENT, IntPtr.Zero, cwd, ref si, out pi))
            throw new System.ComponentModel.Win32Exception();
        DeleteProcThreadAttributeList(attrs);
        return Wait(pi);
    }
}
'@

$root = Join-Path ([IO.Path]::GetTempPath()) ("esc591-maquette-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
$prive = Join-Path $root 'prive'
$public = Join-Path $root 'public'
New-Item -ItemType Directory $prive, $public | Out-Null
# Sans héritage : le dossier temporaire de ce poste accorde « Tous les packages d'application »,
# que tout AppContainer porte (voir WINDOWS-ISOLATION.md). Le dossier de la maquette ne garde que
# l'utilisateur, SYSTEM et les administrateurs, comme un dossier du profil.
$rootAcl = Get-Acl $root
$rootAcl.SetAccessRuleProtection($true, $false)
foreach ($who in @([Security.Principal.WindowsIdentity]::GetCurrent().User.Value, 'S-1-5-18', 'S-1-5-32-544')) {
    $rootAcl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.SecurityIdentifier]$who,
            'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
}
Set-Acl $root $rootAcl
Set-Content (Join-Path $prive 'secret.txt') 'SECRET-DU-PROJET-PRIVE'
Set-Content (Join-Path $public 'ok.txt') 'FICHIER-DU-PROJET'

# Le SID de l'ensemble : un SID inventé, qui n'existe dans aucun compte, sous l'autorité 9 (« gestionnaire
# de ressources »). CreateRestrictedToken refuse un SID de capacité S-1-15-3 comme SID restrictif
# (« Paramètre incorrect ») ; icacls, qui cherche le nom d'un SID, refuse un SID inventé : .NET le pose.
$setSid = 'S-1-9-591-2026-1'
$containerName = 'escouade.maquette.esc591.conteneur'
$containerSid = [Maquette]::AppContainerSid($containerName)
$profileMade = $false

# Un écouteur TCP brut (pas http.sys, qui demanderait une réservation d'adresse).
$listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, $port)
$listener.Start()
$port = $listener.LocalEndpoint.Port
$serve = Start-ThreadJob -ArgumentList $listener {
    param($l)
    while ($true) {
        try {
            $c = $l.AcceptTcpClient(); $s = $c.GetStream(); $buf = New-Object byte[] 4096; [void]$s.Read($buf, 0, $buf.Length)
            $b = [Text.Encoding]::ASCII.GetBytes("HTTP/1.0 200 OK`r`nContent-Length: 9`r`n`r`nMCP-LOCAL"); $s.Write($b, 0, $b.Length); $c.Close()
        } catch { break }
    }
}

$results = [ordered]@{}
function Test-Case($name, $runner) {
    $out = Join-Path $public "$name.txt"
    $code = & $runner $out
    $text = if (Test-Path $out) { "$(Get-Content $out -Raw)".Trim() } else { '(rien écrit)' }
    if (-not $text) { $text = '(sortie vide)' }
    $results[$name] = "code $code : $text"
}

try {
    # R : refus de lecture du dossier privé pour le SID de l'ensemble, hérité par ce qui est dessous.
    $acl = Get-Acl $prive
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.SecurityIdentifier]$setSid,
            'Modify', 'ContainerInherit,ObjectInherit', 'None', 'Deny'))
    Set-Acl $prive $acl
    $cmd = "$env:SystemRoot\System32\cmd.exe"
    $curl = "$env:SystemRoot\System32\curl.exe"
    Test-Case 'R-lit-prive' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c type `"$prive\secret.txt`" > `"$o`" 2>&1", $public, $setSid) }
    Test-Case 'R-lit-public' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c type `"$public\ok.txt`" > `"$o`" 2>&1", $public, $setSid) }
    Test-Case 'R-pwsh-prive' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c powershell -NoProfile -Command `"Get-Content -LiteralPath '$prive\secret.txt'`" > `"$o`" 2>&1", $public, $setSid) }
    Test-Case 'R-boucle-locale' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c `"`"$curl`" -s -m 5 http://127.0.0.1:$port/ > `"$o`" 2>&1`"", $public, $setSid) }
    $git = (Get-Command git -ErrorAction SilentlyContinue).Source
    if ($git) {
        Test-Case 'R-git' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c `"`"$git`" --version > `"$o`" 2>&1`"", $public, $setSid) }
    }
    Test-Case 'R-ecrit-prive' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c echo x > `"$prive\ajout.txt`" 2> `"$o`"", $public, $setSid) }
    if ($git) {
        & $git -C $public init -q 2>&1 | Out-Null
        Test-Case 'R-git-status' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c `"`"$git`" -C `"$public`" status --short > `"$o`" 2>&1`"", $public, $setSid) }
    }
    $results['R-ecrit-prive'] += ' ; fichier écrit : ' + (Test-Path (Join-Path $prive 'ajout.txt'))
    # Un processus ordinaire de l'utilisateur, ouvert pour écrire dans sa mémoire et y lancer un fil :
    # s'il s'ouvre, le jeton restreint s'en échappe (le code injecté y tourne avec le jeton entier).
    $probe = Join-Path $public 'ouvre.ps1'
    # Add-Type compile dans le dossier temporaire, que les jetons R2 et R3 n'écrivent pas : le
    # dossier public (accordé à R2, étiqueté « bas » pour R3) en tient lieu.
    Set-Content $probe (@'
$env:TMP = $env:TEMP = 'DOSSIER_PUBLIC'
Add-Type -Namespace W -Name K -MemberDefinition '[DllImport("kernel32.dll", SetLastError=true)] public static extern System.IntPtr OpenProcess(uint a, bool i, int p);'
$h = [W.K]::OpenProcess(0x002A, $false, PID_CIBLE)
if ($h -isnot [System.IntPtr]) { 'ESSAI MANQUE' }
elseif ($h -ne [System.IntPtr]::Zero) { 'OUVERT (VM_WRITE, CREATE_THREAD)' }
else { 'REFUSE, erreur ' + [System.Runtime.InteropServices.Marshal]::GetLastWin32Error() }
'@).Replace('PID_CIBLE', "$PID").Replace('DOSSIER_PUBLIC', $public)
    Test-Case 'R-ouvre-processus' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c powershell -NoProfile -ExecutionPolicy Bypass -File `"$probe`" > `"$o`" 2>&1", $public, $setSid) }
    # La même chose sans le SID de l'utilisateur dans la liste restrictive (sa session y reste) : le
    # dossier public doit alors être accordé au SID de l'ensemble.
    $pubAcl = Get-Acl $public
    $pubAcl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.SecurityIdentifier]$setSid,
            'Modify', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
    Set-Acl $public $pubAcl
    Test-Case 'R2-lit-public' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c type `"$public\ok.txt`" > `"$o`" 2>&1", $public, $setSid, $false) }
    Test-Case 'R2-ouvre-processus' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c powershell -NoProfile -ExecutionPolicy Bypass -File `"$probe`" > `"$o`" 2>&1", $public, $setSid, $false) }
    Test-Case 'R2-git' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c `"`"$git`" --version > `"$o`" 2>&1`"", $public, $setSid, $false) }
    Test-Case 'R2-profil' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c dir /b `"$env:USERPROFILE`" > `"$o`" 2>&1", $public, $setSid, $false) }
    Test-Case 'R2-boucle-locale' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c `"`"$curl`" -s -m 5 http://127.0.0.1:$port/ > `"$o`" 2>&1`"", $public, $setSid, $false) }
    # R3 : comme R (le SID de l'utilisateur dans la liste), en faible intégrité. Le dossier public
    # porte l'étiquette « bas » pour que l'agent y écrive ; ses résultats y sont écrits.
    icacls $public /setintegritylevel '(OI)(CI)low' | Out-Null
    Test-Case 'R3-lit-prive' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c type `"$prive\secret.txt`" > `"$o`" 2>&1", $public, $setSid, $true, $true) }
    Test-Case 'R3-lit-public' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c type `"$public\ok.txt`" > `"$o`" 2>&1", $public, $setSid, $true, $true) }
    Test-Case 'R3-ouvre-processus' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c powershell -NoProfile -ExecutionPolicy Bypass -File `"$probe`" > `"$o`" 2>&1", $public, $setSid, $true, $true) }
    Test-Case 'R3-git-status' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c `"`"$git`" -C `"$public`" status --short > `"$o`" 2>&1`"", $public, $setSid, $true, $true) }
    Test-Case 'R3-boucle-locale' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c `"`"$curl`" -s -m 5 http://127.0.0.1:$port/ > `"$o`" 2>&1`"", $public, $setSid, $true, $true) }
    $medium = Join-Path $root 'moyen'; New-Item -ItemType Directory $medium | Out-Null
    Test-Case 'R3-ecrit-ailleurs' { param($o) [Maquette]::RunRestricted("`"$cmd`" /c echo x > `"$medium\x.txt`" 2> `"$o`"", $public, $setSid, $true, $true) }
    $results['R3-ecrit-ailleurs'] += ' ; fichier écrit : ' + (Test-Path (Join-Path $medium 'x.txt'))
    # Des outils réels, s'ils sont là : leur version seulement (aucun compte, aucun identifiant lu).
    foreach ($tool in @(
            @{ n = 'R3-node'; c = 'node'; a = '-e "console.log(process.version, require(''os'').tmpdir())"' },
            @{ n = 'R3-claude'; c = 'claude'; a = '--version' },
            @{ n = 'R3-codex'; c = 'codex'; a = '--version' })) {
        $exe = (Get-Command $tool.c -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1).Source
        if (-not $exe) { continue }
        $line = "`"$cmd`" /c set `"TEMP=$public`" && set `"TMP=$public`" && `"$exe`" $($tool.a) > `"$public\$($tool.n).txt`" 2>&1"
        Test-Case $tool.n { param($o) [Maquette]::RunRestricted($line, $public, $setSid, $true, $true) }
    }
    # Le contrôle : l'utilisateur lui-même lit toujours le dossier privé.
    $results['utilisateur-lit-prive'] = (Get-Content (Join-Path $prive 'secret.txt') -Raw).Trim()

    # A : le dossier public seul ouvert au conteneur (lecture, écriture pour ses résultats).
    $hr = [Maquette]::CreateProfile($containerName)
    if ($hr -ne 0) { throw "CreateAppContainerProfile : 0x{0:X8}" -f $hr }
    $profileMade = $true
    icacls $public /grant "*${containerSid}:(OI)(CI)(M)" | Out-Null
    icacls $root /grant "*${containerSid}:(RX)" | Out-Null
    Test-Case 'A-lit-prive' { param($o) [Maquette]::RunAppContainer("`"$cmd`" /c type `"$prive\secret.txt`" > `"$o`" 2>&1", $public, $containerSid) }
    Test-Case 'A-lit-public' { param($o) [Maquette]::RunAppContainer("`"$cmd`" /c type `"$public\ok.txt`" > `"$o`" 2>&1", $public, $containerSid) }
    Test-Case 'A-boucle-locale' { param($o) [Maquette]::RunAppContainer("`"$cmd`" /c `"`"$curl`" -s -m 5 http://127.0.0.1:$port/ > `"$o`" 2>&1`"", $public, $containerSid) }
    if ($git) {
        Test-Case 'A-git' { param($o) [Maquette]::RunAppContainer("`"$cmd`" /c `"`"$git`" --version > `"$o`" 2>&1`"", $public, $containerSid) }
    }
    Test-Case 'A-ouvre-processus' { param($o) [Maquette]::RunAppContainer("`"$cmd`" /c powershell -NoProfile -ExecutionPolicy Bypass -File `"$probe`" > `"$o`" 2>&1", $public, $containerSid) }
    Test-Case 'A-profil' { param($o) [Maquette]::RunAppContainer("`"$cmd`" /c dir `"$env:USERPROFILE`" > `"$o`" 2>&1", $public, $containerSid) }
}
finally {
    $listener.Stop()
    Remove-Job $serve -Force -ErrorAction SilentlyContinue
    if ($profileMade) { $results['profil-retire'] = 'hr {0}' -f [Maquette]::DeleteProfile($containerName) }
    # Les ACE posées vont avec le dossier : retiré entier.
    Remove-Item -Recurse -Force $root
}

$results.GetEnumerator() | ForEach-Object { '{0,-24} {1}' -f $_.Key, $_.Value }
'Dossier de la maquette retiré : ' + -not (Test-Path $root)
$mapping = "HKCU:\Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppContainer\Mappings\$containerSid"
'Profil du conteneur absent du registre : ' + -not (Test-Path -LiteralPath $mapping)
'Dossier du conteneur absent : ' + -not (Test-Path (Join-Path $env:LOCALAPPDATA "Packages\$containerName"))
