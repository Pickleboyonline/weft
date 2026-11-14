import unittest

import weft as w


def tiny():
    app = w.App("t")
    a = app.location("a", caps=["storage"])
    b = app.location("b", replicas=2, caps=["storage", "flash"])

    class Ping(w.Message):
        n: w.Int

    return app, a, b, Ping


class TraceTest(unittest.TestCase):
    def test_handler_becomes_guarded_ssa(self):
        app, a, b, Ping = tiny()
        seen = b.state("seen", w.Int, init=0)

        @b.on(Ping)
        def on_ping(msg):
            with w.when(msg.n > seen.read()):
                seen.write(msg.n)

        ops = app.to_ir()["handlers"][0]["body"]
        self.assertEqual([o["op"] for o in ops], ["field", "read", "bin", "write"])
        self.assertEqual(ops[2]["fn"], "gt")
        self.assertEqual(ops[3]["guard"], 2)
        self.assertNotIn("guard", ops[2])  # pure ops are never guarded

    def test_nested_when_ands_the_guards(self):
        app, a, b, Ping = tiny()
        seen = b.state("seen", w.Int, init=0)

        @b.on(Ping)
        def on_ping(msg):
            with w.when(msg.n > 0):
                with w.when(msg.n < 10):
                    seen.write(msg.n)

        ops = app.to_ir()["handlers"][0]["body"]
        write = ops[-1]
        guard = ops[write["guard"]]
        self.assertEqual(guard["fn"], "and")

    def test_python_if_on_traced_value_is_an_error(self):
        app, a, b, Ping = tiny()
        with self.assertRaisesRegex(w.TracingError, "Python control flow"):

            @b.on(Ping)
            def bad(msg):
                if msg.n > 1:
                    pass

    def test_reading_another_locations_state_is_an_error(self):
        app, a, b, Ping = tiny()
        x = a.state("x", w.Int, init=0)
        with self.assertRaisesRegex(w.TracingError, "share no memory"):

            @b.on(Ping)
            def bad(msg):
                x.read()

    def test_extern_without_sim_model_is_an_error(self):
        app, a, b, Ping = tiny()
        with self.assertRaisesRegex(w.TracingError, "simulation"):

            @b.extern(effects=["flash"])
            def flash(v: w.Int) -> w.Bool: ...

    def test_property_quantifies_over_the_replicated_location(self):
        app, a, b, Ping = tiny()
        seen = b.state("seen", w.Int, init=0)
        total = a.state("total", w.Int, init=0)

        @app.always
        def bounded():
            return seen.read() <= total.read()

        self.assertEqual(app.to_ir()["properties"][0]["forall"], "b")

    def test_sender_only_in_message_handlers(self):
        app, a, b, Ping = tiny()
        with self.assertRaisesRegex(w.TracingError, "sender"):

            @a.on_input(Ping)
            def bad(msg):
                w.send(w.sender(), Ping(n=1))


if __name__ == "__main__":
    unittest.main()
