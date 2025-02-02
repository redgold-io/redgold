<template>
  <div class="container-fluid">
    <nav class="navbar custom-navbar">
      <div class="navbar-container">
        <div class="nav-items">
          <a class="navbar-brand brand d-flex align-items-center" href="/">
            <img :src="require('@/assets/logo.png')" alt="Logo" class="logo">
            <span class="redgold-title">Red</span>
            <span class="redgold-title2">gold</span>
          </a>
          <a class="navbar-brand brand" href="https://dev.explorer.redgold.io">Explorer</a>
          <a class="navbar-brand brand" href="https://dev.docs.redgold.io">Docs</a>
          <a class="navbar-brand brand" href="https://discord.gg/86fzxJg8ce">Discord</a>
          <a class="navbar-brand brand" href="https://github.com/redgold-io/redgold">GitHub</a>
          <a class="navbar-brand brand" href="https://blog.redgold.io">Blog</a>
        </div>
      </div>
    </nav>
  </div>
</template>

<script>
export default {
  name: 'HeaderBox',
}
</script>

<style scoped>
.container-fluid {
  background-color: #29220c;
}

.navbar-container {
  width: 100%;
  padding: 0 1rem;
}
.nav-items {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: .5rem;
  width: 100%;
  padding: 0; /* Removed vertical padding */
}

.logo {
  height: 55px;
  width: 55px;
  margin-right: 5px;
}

.custom-navbar .brand {
  color: #fff;
  user-select: none;
  text-decoration: none;
  padding: 0.25rem 0.1rem; /* Reduced vertical padding */
  white-space: nowrap;
}

.custom-navbar .brand:hover {
  color: #fff;
}

.redgold-title, .redgold-title2 {
  font-size: 1.0em;
  font-weight: bold;
}


@media (max-width: 768px) {
  .nav-items {
    justify-content: flex-start;
  }

  .logo {
    height: 40px;
    width: 40px;
  }

  .custom-navbar .brand {
    padding: 0.1rem 0.5rem; /* Reduced mobile padding */
  }
}

/* Add padding to entire nav container on wider screens */
@media (min-width: 800px) {
  .nav-items {
    padding-left: 175px;
  }
}
</style>