#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>

#include <cstdarg>
#include <cstdio>
#include <cstring>

static DWORD g_target_ip = 0;
static DWORD g_target_mask = 0;
static bool g_active = false;
static bool g_logged_bind = false;
static bool g_logged_broadcast = false;

static void BridgeLog(const char* fmt, ...) {
    FILE* f = fopen("amax\\blur_lan_bridge.log", "a");
    if (!f) return;
    va_list args;
    va_start(args, fmt);
    vfprintf(f, fmt, args);
    va_end(args);
    fputc('\n', f);
    fclose(f);
}

static bool LoadConfig() {
    char gate[32] = {0};
    DWORD n = GetEnvironmentVariableA("BLUR_LAN_BRIDGE", gate, sizeof(gate));
    if (n == 0 || n >= sizeof(gate) || strcmp(gate, "1") != 0) {
        return false;
    }

    FILE* f = fopen("amax\\blur_lan_bridge.cfg", "r");
    if (!f) {
        BridgeLog("config missing: amax\\blur_lan_bridge.cfg");
        return false;
    }

    char line[256];
    while (fgets(line, sizeof(line), f)) {
        char* eq = strchr(line, '=');
        if (!eq) continue;
        *eq = '\0';
        char* key = line;
        char* val = eq + 1;
        while (*key == ' ' || *key == '\t') key++;
        while (*val == ' ' || *val == '\t') val++;
        size_t len = strlen(val);
        while (len && (val[len - 1] == '\n' || val[len - 1] == '\r' || val[len - 1] == ' ')) {
            val[--len] = '\0';
        }
        if (strcmp(key, "address") == 0) g_target_ip = inet_addr(val);
        else if (strcmp(key, "mask") == 0) g_target_mask = inet_addr(val);
    }
    fclose(f);

    if (g_target_ip == 0 || g_target_ip == INADDR_NONE || g_target_mask == 0) {
        BridgeLog("config invalid: address=%08x mask=%08x", g_target_ip, g_target_mask);
        return false;
    }
    return true;
}

typedef int(WSAAPI* BindFn)(SOCKET, const struct sockaddr*, int);
typedef int(WSAAPI* SendToFn)(SOCKET, const char*, int, int, const struct sockaddr*, int);
typedef int(WSAAPI* WSASendToFn)(SOCKET, LPWSABUF, DWORD, LPDWORD, DWORD,
                                 const struct sockaddr*, int, LPWSAOVERLAPPED,
                                 LPWSAOVERLAPPED_COMPLETION_ROUTINE);

static BindFn RealBind = nullptr;
static SendToFn RealSendTo = nullptr;
static WSASendToFn RealWSASendTo = nullptr;

static bool IsLanBind(const sockaddr_in* in) {
    return in->sin_port == htons(50001);
}

static int WSAAPI HookedBind(SOCKET s, const struct sockaddr* name, int namelen) {
    if (g_active && name && name->sa_family == AF_INET && namelen >= sizeof(sockaddr_in)) {
        const sockaddr_in* in = reinterpret_cast<const sockaddr_in*>(name);
        if (IsLanBind(in) || in->sin_addr.s_addr == INADDR_ANY) {
            struct sockaddr_in local = *in;
            local.sin_addr.s_addr = g_target_ip;
            if (!g_logged_bind) {
                g_logged_bind = true;
                BridgeLog("bind forced to selected LAN adapter");
            }
            return RealBind(s, reinterpret_cast<const sockaddr*>(&local), sizeof(local));
        }
    }
    return RealBind(s, name, namelen);
}

static bool RewriteBroadcast(const struct sockaddr* from, struct sockaddr_in& out) {
    if (!g_active || !from || from->sa_family != AF_INET) return false;
    const sockaddr_in* in = reinterpret_cast<const sockaddr_in*>(from);
    if (in->sin_addr.s_addr != INADDR_BROADCAST) return false;
    out = *in;
    out.sin_addr.s_addr = g_target_ip | (~g_target_mask);
    if (!g_logged_broadcast) {
        g_logged_broadcast = true;
        BridgeLog("LAN broadcast redirected to selected adapter subnet");
    }
    return true;
}

static int WSAAPI HookedSendTo(SOCKET s, const char* buf, int len, int flags,
                               const struct sockaddr* to, int tolen) {
    struct sockaddr_in rewritten;
    if (RewriteBroadcast(to, rewritten)) {
        return RealSendTo(s, buf, len, flags,
                          reinterpret_cast<const sockaddr*>(&rewritten), sizeof(rewritten));
    }
    return RealSendTo(s, buf, len, flags, to, tolen);
}

static int WSAAPI HookedWSASendTo(SOCKET s, LPWSABUF bufs, DWORD count, LPDWORD sent,
                                  DWORD flags, const struct sockaddr* to, int tolen,
                                  LPWSAOVERLAPPED ov, LPWSAOVERLAPPED_COMPLETION_ROUTINE completion) {
    struct sockaddr_in rewritten;
    if (RewriteBroadcast(to, rewritten)) {
        return RealWSASendTo(s, bufs, count, sent, flags,
                             reinterpret_cast<const sockaddr*>(&rewritten), sizeof(rewritten),
                             ov, completion);
    }
    return RealWSASendTo(s, bufs, count, sent, flags, to, tolen, ov, completion);
}

static void* MakeTrampoline(void* original, void* hook) {
    const unsigned char prologue[5] = {0x8B, 0xFF, 0x55, 0x8B, 0xEC};
    unsigned char* base = reinterpret_cast<unsigned char*>(original);
    if (memcmp(base, prologue, sizeof(prologue)) != 0) {
        BridgeLog("unexpected ws2_32 prologue %02x %02x %02x %02x %02x - hook skipped",
                  base[0], base[1], base[2], base[3], base[4]);
        return nullptr;
    }
    unsigned char* tramp = reinterpret_cast<unsigned char*>(
        VirtualAlloc(nullptr, 32, MEM_COMMIT, PAGE_EXECUTE_READWRITE));
    if (!tramp) return nullptr;
    memcpy(tramp, base, 5);
    tramp[5] = 0xE9;
    long rel = reinterpret_cast<unsigned char*>(original) + 5 - (tramp + 10);
    memcpy(tramp + 6, &rel, 4);
    return tramp;
}

static bool InstallHook(HMODULE ws2, const char* name, void* hook, void** originalOut) {
    FARPROC target = GetProcAddress(ws2, name);
    if (!target) {
        BridgeLog("GetProcAddress(%s) failed", name);
        return false;
    }
    void* tramp = MakeTrampoline(reinterpret_cast<void*>(target), hook);
    if (!tramp) return false;

    DWORD oldProtect = 0;
    unsigned char* site = reinterpret_cast<unsigned char*>(target);
    if (!VirtualProtect(site, 5, PAGE_EXECUTE_READWRITE, &oldProtect)) {
        BridgeLog("VirtualProtect(%s) failed", name);
        VirtualFree(tramp, 0, MEM_RELEASE);
        return false;
    }
    long rel = reinterpret_cast<unsigned char*>(hook) - (site + 5);
    site[0] = 0xE9;
    memcpy(site + 1, &rel, 4);
    DWORD ignored = 0;
    VirtualProtect(site, 5, oldProtect, &ignored);
    FlushInstructionCache(GetCurrentProcess(), site, 5);

    *originalOut = tramp;
    BridgeLog("hooked ws2_32!%s", name);
    return true;
}

static DWORD WINAPI BridgeInit(LPVOID) {
    if (!LoadConfig()) {
        BridgeLog("inactive (no launcher config or gate)");
        return 0;
    }
    HMODULE ws2 = GetModuleHandleA("ws2_32.dll");
    if (!ws2) {
        BridgeLog("ws2_32.dll is not loaded yet");
        return 0;
    }
    void* bindTramp = nullptr;
    void* sendToTramp = nullptr;
    void* wsaSendToTramp = nullptr;
    bool ok = InstallHook(ws2, "bind", reinterpret_cast<void*>(HookedBind), &bindTramp) &&
              InstallHook(ws2, "sendto", reinterpret_cast<void*>(HookedSendTo), &sendToTramp) &&
              InstallHook(ws2, "WSASendTo", reinterpret_cast<void*>(HookedWSASendTo), &wsaSendToTramp);
    if (!ok) {
        BridgeLog("hook installation incomplete - bridge disabled");
        return 0;
    }
    RealBind = reinterpret_cast<BindFn>(bindTramp);
    RealSendTo = reinterpret_cast<SendToFn>(sendToTramp);
    RealWSASendTo = reinterpret_cast<WSASendToFn>(wsaSendToTramp);
    g_active = true;
    BridgeLog("active: target %08x mask %08x", g_target_ip, g_target_mask);
    return 0;
}

extern "C" __declspec(dllexport) int plugin_init() {
    return 1;
}

BOOL APIENTRY DllMain(HMODULE module, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) {
        DisableThreadLibraryCalls(module);
        if (GetModuleHandleA("ws2_32.dll")) {
            BridgeInit(nullptr);
        } else {
            CreateThread(nullptr, 0, BridgeInit, nullptr, 0, nullptr);
        }
    }
    return TRUE;
}
