package me.danwi.sqlex.common;

import java.util.HashMap;
import java.util.LinkedList;
import java.util.Map;

/**
 * LRU缓存
 *
 * @param <K> key类型
 * @param <V> value类型
 */
public class LRUCache<K, V> {
    private final int size;
    private final Map<K, Data> cache;
    private final LinkedList<Data> list;

    private class Data {
        K key;
        V value;

        Data(K key, V value) {
            this.key = key;
            this.value = value;
        }
    }

    /**
     * 新建LRU缓存
     *
     * @param size 缓存容量
     */
    public LRUCache(int size) {
        this.size = size;
        this.cache = new HashMap<>(size);
        this.list = new LinkedList<>();
    }

    /**
     * 从缓存中取值,会刷新对应缓存的使用情况
     *
     * @param key 缓存key
     * @return key对应的缓存值, 如果不存在, 则返回null
     */
    public V get(K key) {
        if (cache.containsKey(key)) {
            Data data = cache.get(key);
            //移动到队头
            list.remove(data);
            list.addFirst(data);
            return data.value;
        }
        return null;
    }

    /**
     * 将值放入缓存中
     *
     * @param key   缓存key
     * @param value 缓存value
     */
    public void set(K key, V value) {
        if (cache.containsKey(key)) {
            //更新
            //移除旧的数据
            Data oldData = cache.get(key);
            list.remove(oldData);
            //新数据
            Data newData = new Data(key, value);
            cache.put(key, newData);
            list.addFirst(newData);
        } else {
            Data data = new Data(key, value);
            if (cache.size() >= size) {
                //移除最旧的一个数据
                Data toRemove = list.pollLast();
                if (toRemove != null)
                    cache.remove(toRemove.key);
            }
            cache.put(key, data);
            list.addFirst(data);
        }
    }
}
