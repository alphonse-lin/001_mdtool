# mdtool 示例文档

这是一份用来**手测**渲染效果的文档，包含 *斜体*、~~删除线~~、`行内代码` 和自动链接 https://github.com 。

跳转到 [代码块](#代码块) 或 [数学公式](#数学公式)；打开 [另一个文件](other.md)。

![图标](../res/icon.png)

## 列表

- 第一项
- 第二项
  - 嵌套项
  - 另一个嵌套项
1. 有序一
2. 有序二

### 任务列表

- [x] 写设计文档
- [x] 写实现计划
- [ ] 实测启动速度

## 表格

| 方案 | 启动耗时 | 体积 |
|:---|:---:|---:|
| Python + pywebview | ~1.2s | 20MB |
| Rust + WebView2 | ~0.4s | 5MB |

## 代码块

```python
def fib(n: int) -> int:
    """返回第 n 个斐波那契数"""
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a
```

```rust
fn main() {
    let name = "mdtool";
    println!("hello, {name}!");
}
```

## 数学公式

行内公式：质能方程 $E = mc^2$，以及 $\sum_{i=1}^{n} i = \frac{n(n+1)}{2}$。

块级公式：

$$
\int_{-\infty}^{\infty} e^{-x^2} \, dx = \sqrt{\pi}
$$

## 引用与脚注

> 好的工具应该快到让人感觉不到它的存在。

这里有一个脚注[^1]，还有另一个[^note]。

---

[^1]: 第一个脚注的内容。
[^note]: 第二个脚注，可以写**格式**。
