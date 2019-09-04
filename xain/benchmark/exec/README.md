# Benchmark Execution

To execute a benchmark locally:

```bash
python -m xain.benchmark.exec \
    --group_name=abc \
    --task_name=def \
    --dataset=fashion-mnist-100p-iid-balanced \
    --model=blog_cnn \
    --R=2 \
    --E=2 \
    --C=0.02 \
    --B=64
```

# Plotting

To plot final task accuracies in a group of tasks use

```bash
plot_final_task_accuracies --group_name=GROUP_NAME
```
