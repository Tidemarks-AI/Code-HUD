package com.example.app

import kotlin.collections.List
import kotlin.collections.Map

interface Repository<T> {
    fun findById(id: String): T?
    fun findAll(): List<T>
    fun save(entity: T)
}

data class User(val name: String, val age: Int) {
    fun greet(): String {
        return "Hello, $name!"
    }
}

sealed class Result<out T> {
    data class Success<T>(val data: T) : Result<T>()
    data class Error(val message: String) : Result<Nothing>()
}

enum class Status {
    ACTIVE,
    INACTIVE,
    PENDING;

    fun label(): String = name.lowercase()
}

object AppConfig {
    val version = "1.0.0"

    fun init() {
        println("Initializing...")
    }
}

class UserService(private val repo: Repository<User>) {
    val cache: MutableMap<String, User> = mutableMapOf()

    fun getUser(id: String): User? {
        return cache[id]
    }

    private fun refreshCache() {
        cache.clear()
    }

    companion object {
        fun create(): UserService {
            return UserService(object : Repository<User> {
                override fun findById(id: String): User? = null
                override fun findAll(): List<User> = emptyList()
                override fun save(entity: User) {}
            })
        }
    }
}

fun topLevelFunction(x: Int): Int = x * 2

val topLevelProperty: String = "hello"

typealias StringList = List<String>
