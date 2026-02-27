package com.example.app;

import java.util.List;
import java.util.Map;
import java.io.*;

public class UserService {
    private static final int MAX_USERS = 100;
    private Map<String, String> cache;

    public UserService() {
        this.cache = new java.util.HashMap<>();
    }

    public String getUser(String id) {
        return cache.get(id);
    }

    private void refreshCache() {
        cache.clear();
    }

    public static UserService create() {
        return new UserService();
    }

    protected int userCount() {
        return cache.size();
    }
}

interface Repository<T> {
    T findById(String id);
    List<T> findAll();
    void save(T entity);
}

public enum Status {
    ACTIVE,
    INACTIVE,
    PENDING;

    public String label() {
        return name().toLowerCase();
    }
}

public record Point(int x, int y) {
    public double distance() {
        return Math.sqrt(x * x + y * y);
    }
}

@interface Cacheable {
    int ttl() default 60;
}

class InternalHelper {
    void doWork() {
        System.out.println("working");
    }
}
