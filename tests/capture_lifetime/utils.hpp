#pragma once
#include <algorithm>
#include <cmath>
#include <format>
#include <string>
#include <vector>
namespace rust { using String = std::string; }
namespace SDK {
struct FVector2D { double X = 0, Y = 0; };
struct FLinearColor { float R, G, B, A; };
struct FDrawToRenderTargetContext {};
struct UObject { bool alive = true; };
struct UTexture : UObject {};
struct UTextureRenderTarget2D : UTexture {
    int SizeX = 1920, SizeY = 1080, RenderTargetFormat = 0;
    FLinearColor ClearColor{0, 0, 0, 1};
    std::string GetName() { return "test_target"; }
    std::string GetFullName() { return "TextureRenderTarget2D test_target"; }
};
struct UWidget : UObject { std::wstring Name = L"Image_44"; };
enum class EBlendMode { BLEND_Opaque };
struct FixtureState {
    int allocations = 0, releases = 0, begins = 0, draws = 0, ends = 0;
    int failBegin = 0; bool failAllocation = false;
    FVector2D viewport{3440, 1440};
    std::vector<FVector2D> origins, extents;
};
inline FixtureState fixture;
struct UCanvas : UObject {
    void K2_DrawTexture(UTexture*, FVector2D, FVector2D, FVector2D origin, FVector2D extent,
                        FLinearColor, EBlendMode, double, FVector2D) {
        ++fixture.draws; fixture.origins.push_back(origin); fixture.extents.push_back(extent);
    }
};
struct UKismetStringLibrary { static std::wstring Conv_StringToName(const wchar_t* value) { return value; } };
struct UKismetRenderingLibrary {
    static UTextureRenderTarget2D* CreateRenderTarget2D(UObject*, int x, int y, int format, FLinearColor clear, bool, bool) {
        ++fixture.allocations; if (fixture.failAllocation) return nullptr;
        auto target = new UTextureRenderTarget2D; target->SizeX = x; target->SizeY = y;
        target->RenderTargetFormat = format; target->ClearColor = clear; return target;
    }
    static void ReleaseRenderTarget2D(UTextureRenderTarget2D* target) { ++fixture.releases; delete target; }
    static void BeginDrawCanvasToRenderTarget(UObject*, UTextureRenderTarget2D*, UCanvas** canvas, FVector2D* size, FDrawToRenderTargetContext*) {
        static UCanvas available;
        ++fixture.begins; *canvas = fixture.begins == fixture.failBegin ? nullptr : &available; *size = {1920, 1080};
    }
    static void EndDrawCanvasToRenderTarget(UObject*, FDrawToRenderTargetContext) { ++fixture.ends; }
};
}
namespace ffrs {
using namespace SDK;
inline bool live(UObject* object) { return object && object->alive; }
inline bool approximately(double a, double b) { return std::abs(a - b) < 0.00001; }
inline bool arranged(FVector2D v) { return v.X >= 1 && v.Y >= 1; }
inline FVector2D viewport_local_size(UObject*) { return fixture.viewport; }
inline std::string hud_report(const std::string&, const std::string& text) { return text; }
}
