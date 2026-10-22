#include "utils.hpp"
#include <Windows.h>
#include <atomic>
#include <cstring>

namespace ffrs {
namespace {
std::uintptr_t expected_event = 0;
std::atomic<unsigned> validation{0};
std::atomic_flag checking = ATOMIC_FLAG_INIT;
bool readable(const void* pointer, std::size_t size) {
    if (!pointer || !size) return false;
    auto at = reinterpret_cast<std::uintptr_t>(pointer);
    const auto end = at + size;
    if (end < at) return false;
    while (at < end) {
        MEMORY_BASIC_INFORMATION memory{};
        if (!VirtualQuery(reinterpret_cast<void*>(at), &memory, sizeof(memory))
            || memory.State != MEM_COMMIT || (memory.Protect & (PAGE_NOACCESS | PAGE_GUARD))) return false;
        const auto next = reinterpret_cast<std::uintptr_t>(memory.BaseAddress) + memory.RegionSize;
        if (next <= at) return false;
        at = next;
    }
    return true;
}
bool property(UStruct* type, const char* name, int offset) {
    auto field = type->ChildProperties;
    for (unsigned count = 0; field && count < 1024; ++count, field = field->Next) {
        if (!readable(field, sizeof(FProperty))) return false;
        if (field->Name.ToString() == name) return static_cast<FProperty*>(field)->Offset == offset;
    }
    return false;
}
}

bool initialize_sdk(std::size_t objects, std::size_t append, std::size_t event) noexcept {
    if (!readable(reinterpret_cast<void*>(objects), sizeof(TUObjectArray))
        || !readable(reinterpret_cast<void*>(append), 64)
        || !readable(reinterpret_cast<void*>(event), 64)) return false;
    UObject::GObjects.InitManually(reinterpret_cast<void*>(objects));
    FName::InitManually(reinterpret_cast<void*>(append));
    expected_event = event;
    return true;
}

// Invoked on the engine callback thread before any SDK-based correction.
// Waiting is retryable; a layout mismatch permanently disables callbacks.
std::uint32_t sdk_ready() noexcept {
    auto status = validation.load(std::memory_order_acquire);
    if (status) return status;
    if (checking.test_and_set(std::memory_order_acquire)) return 0;
    struct Unlock { ~Unlock() { checking.clear(std::memory_order_release); } } unlock;
    status = validation.load(std::memory_order_acquire);
    if (status) return status;
    const auto objects = UObject::GObjects.GetTypedPtr();
    if (!objects->Objects || objects->NumElements < 100) return 0;
    if (objects->NumElements > objects->MaxElements || objects->NumChunks < 1
        || objects->NumChunks > objects->MaxChunks || objects->NumChunks > 1024
        || !readable(objects->Objects, objects->NumChunks * sizeof(void*))) {
        validation.store(2); return 2;
    }
    if (!readable(objects->Objects[0], sizeof(FUObjectItem))) {
        validation.store(2); return 2;
    }
    auto first = objects->GetByIndex(0);
    if (!first) return 0;
    if (!readable(first, sizeof(UObject)) || first->Index != 0
        || !readable(first->VTable, (Offsets::ProcessEventIdx + 1) * sizeof(void*))
        || reinterpret_cast<std::uintptr_t>(reinterpret_cast<void**>(first->VTable)[Offsets::ProcessEventIdx]) != expected_event) {
        validation.store(2); return 2;
    }
    struct Check { const char* type; const char* member; int offset; };
    const Check checks[] = {
        {"Widget", "Slot", offsetof(UWidget, Slot)},
        {"Widget", "RenderTransform", offsetof(UWidget, RenderTransform)},
        {"Widget", "RenderTransformPivot", offsetof(UWidget, RenderTransformPivot)},
        {"Widget", "Visibility", offsetof(UWidget, Visibility)},
        {"UserWidget", "WidgetTree", offsetof(UUserWidget, WidgetTree)},
        {"WidgetTree", "RootWidget", offsetof(UWidgetTree, RootWidget)},
        {"PanelWidget", "Slots", offsetof(UPanelWidget, Slots)},
        {"PanelSlot", "Parent", offsetof(UPanelSlot, Parent)},
        {"PanelSlot", "Content", offsetof(UPanelSlot, Content)},
        {"Image", "Brush", offsetof(UImage, Brush)},
        {"CPP_WBP_MovieScreen", "Projector", offsetof(UCPP_WBP_MovieScreen, Projector)},
        {"CPP_WBP_MovieProjector", "Projector", offsetof(UCPP_WBP_MovieProjector, Projector)},
    };
    for (const auto& check : checks) {
        auto type = UObject::FindClassFast(check.type);
        if (!type) return 0;
        if (!property(type, check.member, check.offset)) {
            validation.store(2); return 2;
        }
    }
    validation.store(1, std::memory_order_release);
    return 1;
}
} // namespace ffrs
