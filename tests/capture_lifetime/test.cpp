#include "encounter-under-test.cpp"
#include <iostream>
#include <stdexcept>
using namespace SDK;
static void require(bool value, const char* text) { if (!value) throw std::runtime_error(text); }
static void capture(UObject& context, UTextureRenderTarget2D& target) {
    ffrs::note_capture_target(reinterpret_cast<std::size_t>(&context), reinterpret_cast<std::size_t>(&target));
}
int main() try {
    UObject context; UTextureRenderTarget2D target; UWidget image;
    auto run = [&]() { return ffrs::reframe_capture(reinterpret_cast<std::size_t>(&image)); };
    fixture = {}; capture(context, target);
    require(!run().empty(), "successful capture must report correction");
    require(fixture.allocations == 1 && fixture.releases == 1 && fixture.draws == 2, "success must release exactly one temporary target");
    const auto keep = (1920.0 / 1080.0) * 1440.0 / 3440.0;
    require(ffrs::approximately(fixture.extents[1].X, keep) && ffrs::approximately(fixture.origins[1].X, (1 - keep) / 2), "crop geometry changed");
    run(); require(fixture.allocations == 1, "same capture must not repeat");
    capture(context, target); run();
    require(fixture.allocations == 2 && fixture.releases == 2, "next transition must use a fresh target after prior destruction");
    fixture = {}; fixture.failBegin = 1; capture(context, target);
    require(run().empty() && fixture.releases == 1 && fixture.draws == 0, "first draw failure must release target");
    run(); require(fixture.allocations == 1, "failure must not allocate again every widget update");
    fixture = {}; fixture.failBegin = 2; capture(context, target);
    require(run().empty() && fixture.releases == 1 && fixture.draws == 1, "second draw failure must release target");
    fixture = {}; fixture.failAllocation = true; capture(context, target);
    require(run().empty() && fixture.allocations == 1 && fixture.releases == 0, "allocation failure must be safe");
    fixture = {}; fixture.viewport = {1920, 1080}; capture(context, target);
    require(run().empty() && fixture.allocations == 0, "16:9 must not allocate scratch");
    std::cout << "PASS: actual encounter.cpp: fresh lifetime per transition, two-draw crop, single attempt, allocation/draw failure cleanup, 16:9 no-op\n";
} catch (const std::exception& error) {
    std::cerr << "FAIL: " << error.what() << '\n'; return 1;
}
