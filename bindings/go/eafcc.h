typedef struct eafcc_CFGCenter eafcc_CFGCenter;

typedef struct eafcc_Context eafcc_Context;

typedef struct {
  char *content_type;
  char *value;
} eafcc_ConfigValue;

const eafcc_CFGCenter *new_config_center_client(const char *cfg);

const eafcc_Context *new_context(const char *val);

void free_context(eafcc_Context *ctx);

eafcc_ConfigValue *get_config(eafcc_CFGCenter *cc, eafcc_Context *ctx, char *key);

void free_config_value(eafcc_ConfigValue *v);
