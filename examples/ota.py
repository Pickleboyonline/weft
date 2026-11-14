"""Hello world for Weft: an over-the-air (OTA) firmware rollout.

    operator --Release--> cloud --Offer--> device x5 --> flash_write (extern)
                            ^                  |
                            +--- Heartbeat ----+   every 5s; the cloud re-offers to stragglers

    make demo        # build the IR, check it, run one seed, sweep 500 seeds
"""

import weft as w


def build(name: str = "ota", *, buggy: bool = False) -> w.App:
    app = w.App(name)

    # Locations are capability sets, not device types. A "device" is any place
    # with a clock, local storage and a flash chip.
    cloud = app.location("cloud", caps=["storage"])
    device = app.location("device", replicas=5, caps=["storage", "clock", "flash"])

    class Release(w.Message):  # from the operator, outside the system
        version: w.Int

    class Offer(w.Message):  # cloud -> device
        version: w.Int

    class Heartbeat(w.Message):  # device -> cloud
        installed: w.Int

    latest = cloud.state("latest", w.Int, init=1)
    installed = device.state("installed", w.Int, init=1)

    # Escape hatch. Weft can't see inside flash_write, so it must declare its effect
    # (it needs the `flash` capability) and how it behaves in simulation.
    @device.extern(effects=["flash"], sim=w.sim.fallible(fail_rate=0.3, latency_ms=(200, 1_500)))
    def flash_write(version: w.Int) -> w.Bool:
        """Download, write and verify a firmware image. Real implementation: platform code."""

    @cloud.on_input(Release)
    def release(msg):
        with w.when(msg.version > latest.read()):
            latest.write(msg.version)
            w.send(device, Offer(version=msg.version))  # every device

    @cloud.on(Heartbeat)
    def on_heartbeat(msg):
        # Re-offer to stragglers: this is what makes the rollout survive dropped
        # messages and failed flashes.
        with w.when(msg.installed < latest.read()):
            w.send(w.sender(), Offer(version=latest.read()))

    @device.on(Offer)
    def on_offer(msg):
        # A plain Python `if` on a build-time value (`buggy`) is fine: it picks which
        # graph to build. On a traced value (`msg.version`) it is an error.
        if buggy:
            newer = msg.version != installed.read()  # BUG: a late, older Offer downgrades the device
        else:
            newer = msg.version > installed.read()
        with w.when(newer):
            ok = flash_write(msg.version)
            with w.when(ok):
                installed.write(msg.version)

    @device.every(ms=5_000)
    def heartbeat():
        w.send(cloud, Heartbeat(installed=installed.read()))

    # ---- what the simulator checks

    app.monotonic(installed, name="never_downgrades")

    @app.always  # every device, after every event
    def never_ahead_of_cloud():
        return installed.read() <= latest.read()

    @app.eventually  # every device, at the end of the run
    def fleet_converges():
        return installed.read() == latest.read()

    # ---- a test scenario (harness only; never deployed)
    # Two releases 300ms apart over a lossy network whose latency spread (5ms-2s)
    # reorders messages. The network heals at 60s.
    app.scenario(
        "two_releases",
        duration_ms=180_000,
        heal_at_ms=60_000,
        network=w.network(drop_rate=0.1, latency_ms=(5, 2_000)),
        inputs=[
            w.at(1_000, cloud, Release(version=2)),
            w.at(1_300, cloud, Release(version=3)),
        ],
    )
    return app


app = build()
app_buggy = build("ota_buggy", buggy=True)
