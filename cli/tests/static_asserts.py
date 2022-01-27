from unittest import TestCase

# Dummy TestCase instance, so we can initialize an instance
# and access the assert instance methods
class DummyTestCase(TestCase):
    def __init__(self):
        super(DummyTestCase, self).__init__()

# A metaclass that makes __getattr__ static
class AssertsAccessorType(type):
    dummy = DummyTestCase()

    def __getattr__(cls, key):
        return getattr(AssertsAccessor.dummy, key)

# The actual accessor, a static class, that redirect the asserts
class AssertsAccessor(object, metaclass=AssertsAccessorType):
    pass

