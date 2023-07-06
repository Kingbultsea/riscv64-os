import os

base_address = 0x80400000
step = 0x20000
linker = 'src/linker.ld'

app_id = 0
apps = os.listdir('src/bin')
apps.sort()
for app in apps:
    # 截取从字符串开头到第一个出现的点号（.）之前的部分，cargo build --bin xxxx  不需要.后缀
    app = app[:app.find('.')]
    lines = []
    lines_before = []
    with open(linker, 'r') as f:
        for line in f.readlines():
            lines_before.append(line)
            line = line.replace(hex(base_address), hex(base_address+step*app_id))
            lines.append(line)
    with open(linker, 'w+') as f:
        f.writelines(lines)
    # 该命令会删除被修改后的linker.ld
    os.system('cargo build --bin %s --release' % app)
    print('[build.py] application %s start with address %s' %(app, hex(base_address+step*app_id)))
    # 恢复linker.ld
    with open(linker, 'w+') as f:
        f.writelines(lines_before)
    app_id = app_id + 1
