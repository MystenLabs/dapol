# https://scikit-spatial.readthedocs.io/en/stable/gallery/fitting/plot_plane.html
# this does not plot for some reason

# from skspatial.objects import Plane, Points
# from skspatial.plotting import plot_3d


# points = Points([[0, 0, 0], [1, 3, 5], [-5, 6, 3], [3, 6, 7], [-2, 6, 7]])

# plane = Plane.best_fit(points)


# plot_3d(
#     points.plotter(c='k', s=50, depthshade=False),
#     plane.plotter(alpha=0.2, lims_x=(-5, 5), lims_y=(-5, 5)),
# )

# ///////////////////////////////////////////////////////////
# https://stackoverflow.com/questions/56439930/how-to-use-the-datasets-to-fit-the-3d-surface

# TODO we need to rather fit an arbitrary surface than a plane.
# (see data in results/memory directory)

# If you look at the points in bench_data_combined.csv then you can clearly see that a
# plane is not the best way to fit the data. Better would be just to fit it using some
# number of iterations of the Taylor series. The above link seems to show this.

# ///////////////////////////////////////////////////////////
# https://math.stackexchange.com/questions/99299/best-fitting-plane-given-a-set-of-points

import matplotlib.pyplot as plt
from matplotlib import cm
import numpy as np
import pandas as pd
import sys

# These constants are to create random data for the sake of this example
N_POINTS = 10
TARGET_X_SLOPE = 2
TARGET_y_SLOPE = 3
TARGET_OFFSET  = 5
EXTENTS = 5
NOISE = 5

csvfile = pd.read_csv(sys.argv[1], dtype=np.float64)
xs = csvfile["height"].to_list()
ys = csvfile["num_entities"].to_list()
zs = csvfile["memory(MB)"].to_list()

# Create random data.
# In your solution, you would provide your own xs, ys, and zs data.
# xs = [np.random.uniform(2*EXTENTS)-EXTENTS for i in range(N_POINTS)]
# ys = [np.random.uniform(2*EXTENTS)-EXTENTS for i in range(N_POINTS)]
# zs = []
# for i in range(N_POINTS):
#     zs.append(xs[i]*TARGET_X_SLOPE + \
#               ys[i]*TARGET_y_SLOPE + \
#               TARGET_OFFSET + np.random.normal(scale=NOISE))

# plot raw data
plt.figure()
ax = plt.subplot(111, projection='3d')
ax.scatter(xs, ys, zs, color='b')

# do fit
tmp_A = []
tmp_b = []
for i in range(len(xs)):
    tmp_A.append([xs[i], ys[i], 1])
    tmp_b.append(zs[i])
b = np.matrix(tmp_b).T
A = np.matrix(tmp_A)

# Manual solution
fit = (A.T * A).I * A.T * b
errors = b - A * fit
residual = np.linalg.norm(errors)

# Or use Scipy
# from scipy.linalg import lstsq
# fit, residual, rnk, s = lstsq(A, b)

print("solution: %f x + %f y + %f = z" % (fit[0], fit[1], fit[2]))
print("errors: \n", errors)
print("residual:", residual)

# Make data
X = np.array(xs)
Y = np.array(ys)
X, Y = np.meshgrid(X, Y)
a = np.repeat(fit[0], len(xs)*len(ys)).reshape(X.shape)
b = np.repeat(fit[1], len(xs)*len(ys)).reshape(X.shape)
c = np.repeat(fit[2], len(xs)*len(ys)).reshape(X.shape)
Z = np.multiply(a, X) + np.multiply(b, Y) + c

# Plot the surface
#ax.plot_surface(X, Y, Z, cmap=cm.Blues)
ax.set_xlabel('Height')
ax.set_ylabel('Number of entities')
ax.set_zlabel('Memory (MB)')

plt.show()
