package me.danwi.sqlex.common;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

public class LRUCacheTest {
    @Test
    void cacheTest() {
        LRUCache<String, String> cache = new LRUCache<>(5);
        cache.set("1", "1");
        cache.set("2", "2");
        cache.set("3", "3");
        cache.set("4", "4");
        cache.set("5", "5");

        assertNotNull(cache.get("1"));
        assertNotNull(cache.get("5"));

        cache.set("6", "6");
        assertNotNull(cache.get("6"));
        assertNull(cache.get("2"));

        cache.set("7", "7");
        assertNotNull(cache.get("7"));
        assertNull(cache.get("3"));

        cache.set("8", "8");
        assertNotNull(cache.get("8"));
        assertNull(cache.get("4"));

        cache.set("9", "9");
        assertNotNull(cache.get("9"));
        assertNull(cache.get("1"));

        cache.set("10", "10");
        assertNotNull(cache.get("10"));
        assertNull(cache.get("5"));

        cache.set("11", "11");
        assertNotNull(cache.get("11"));
        assertNull(cache.get("6"));

        cache.set("11", "12");
        assertNotNull(cache.get("11"));
        assertEquals("7", cache.get("7"));
    }
}
